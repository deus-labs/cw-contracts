use cosmwasm_std::{
    attr, coin, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use crate::error::ContractError;
use crate::helper::extract_budget_coin;
use crate::matching::{calculate_clr, QuadraticFundingAlgorithm, RawGrant};
use crate::msg::{AllProposalsResponse, ExecuteMsg, InitMsg, QueryMsg};
use crate::state::{Config, Proposal, Vote, CONFIG, PROPOSALS, PROPOSAL_SEQ, VOTES};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn init(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    msg.validate(env)?;

    let budget = extract_budget_coin(info.funds.as_slice(), &msg.budget_denom)?;
    let mut create_proposal_whitelist: Option<Vec<String>> = None;
    let mut vote_proposal_whitelist: Option<Vec<String>> = None;
    if let Some(pwl) = msg.create_proposal_whitelist {
        let mut tmp_wl = vec![];
        for w in pwl {
            deps.api.addr_validate(&w)?;
            tmp_wl.push(w);
        }
        create_proposal_whitelist = Some(tmp_wl);
    }
    if let Some(vwl) = msg.vote_proposal_whitelist {
        let mut tmp_wl = vec![];
        for w in vwl {
            deps.api.addr_validate(&w)?;
            tmp_wl.push(w);
        }
        vote_proposal_whitelist = Some(tmp_wl);
    }

    let cfg = Config {
        admin: msg.admin,
        leftover_addr: msg.leftover_addr,
        create_proposal_whitelist,
        vote_proposal_whitelist,
        voting_period: msg.voting_period,
        proposal_period: msg.proposal_period,
        algorithm: msg.algorithm,
        budget,
    };
    CONFIG.save(deps.storage, &cfg)?;
    PROPOSAL_SEQ.save(deps.storage, &0)?;

    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateProposal {
            title,
            description,
            metadata,
            fund_address,
        } => execute_create_proposal(deps, env, info, title, description, metadata, fund_address),
        ExecuteMsg::VoteProposal { proposal_id } => {
            execute_vote_proposal(deps, env, info, proposal_id)
        }
        ExecuteMsg::TriggerDistribution { .. } => execute_trigger_distribution(deps, env, info),
    }
}

pub fn execute_create_proposal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    description: String,
    metadata: Option<Binary>,
    fund_address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check whitelist
    if let Some(wl) = config.create_proposal_whitelist {
        if !wl.contains(&info.sender.to_string()) {
            return Err(ContractError::Unauthorized {});
        }
    }

    // check proposal expiration
    if config.proposal_period.is_expired(&env.block) {
        return Err(ContractError::ProposalPeriodExpired {});
    }

    // validate fund address
    deps.api.addr_validate(fund_address.as_str())?;

    let id = PROPOSAL_SEQ.load(deps.storage)? + 1;
    PROPOSAL_SEQ.save(deps.storage, &id)?;
    let p = Proposal {
        id,
        title: title.clone(),
        description,
        metadata,
        fund_address,
        ..Default::default()
    };
    PROPOSALS.save(deps.storage, id.into(), &p)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "create_proposal"),
        attr("title", title),
        attr("proposal_id", id.to_string()),
    ]))
}

pub fn execute_vote_proposal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check whitelist
    if let Some(wl) = config.vote_proposal_whitelist {
        if !wl.contains(&info.sender.to_string()) {
            return Err(ContractError::Unauthorized {});
        }
    }

    // check voting expiration
    if config.voting_period.is_expired(&env.block) {
        return Err(ContractError::VotingPeriodExpired {});
    }

    // validate sent funds and funding denom matches
    let fund = extract_budget_coin(&info.funds, &config.budget.denom)?;

    // check existence of the proposal and collect funds in proposal
    let proposal = PROPOSALS.update(deps.storage, proposal_id.into(), |op| match op {
        None => Err(ContractError::ProposalNotFound {}),
        Some(mut proposal) => {
            proposal.collected_funds += fund.amount;
            Ok(proposal)
        }
    })?;

    let vote = Vote {
        proposal_id,
        voter: info.sender.to_string(),
        fund,
    };

    // check sender did not voted on proposal
    let vote_key = VOTES.key((proposal_id.into(), info.sender.as_bytes()));
    if vote_key.may_load(deps.storage)?.is_some() {
        return Err(ContractError::AddressAlreadyVotedProject {});
    }

    // save vote
    vote_key.save(deps.storage, &vote)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "vote_proposal"),
        attr("proposal_key", proposal_id.to_string()),
        attr("voter", vote.voter),
        attr("collected_fund", proposal.collected_funds),
    ]))
}

pub fn execute_trigger_distribution(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // only admin can trigger distribution
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // check voting period expiration
    if !config.voting_period.is_expired(&env.block) {
        return Err(ContractError::VotingPeriodNotExpired {});
    }

    let query_proposals: StdResult<Vec<_>> = PROPOSALS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();

    let proposals: Vec<Proposal> = query_proposals?.into_iter().map(|p| p.1).collect();

    let mut grants: Vec<RawGrant> = vec![];
    // collect proposals under grants
    for p in proposals {
        let vote_query: StdResult<Vec<(Vec<u8>, Vote)>> = VOTES
            .prefix(p.id.into())
            .range(deps.storage, None, None, Order::Ascending)
            .collect();

        let mut votes: Vec<u128> = vec![];
        for v in vote_query? {
            votes.push(v.1.fund.amount.u128());
        }
        let grant = RawGrant {
            addr: p.fund_address,
            funds: votes,
            collected_vote_funds: p.collected_funds.u128(),
        };

        grants.push(grant);
    }

    let (distr_funds, leftover) = match config.algorithm {
        QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism { .. } => {
            calculate_clr(grants, Some(config.budget.amount.u128()))?
        }
    };

    let mut msgs = vec![];
    for f in distr_funds {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: f.addr,
            amount: vec![coin(f.grant + f.collected_vote_funds, &config.budget.denom)],
        }));
    }

    let leftover_msg: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        to_address: config.leftover_addr,
        amount: vec![coin(leftover, config.budget.denom)],
    });

    msgs.push(leftover_msg);

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "trigger_distribution"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ProposalByID { id } => to_binary(&query_proposal_id(deps, id)?),
        QueryMsg::AllProposals {} => to_binary(&query_all_proposals(deps)?),
    }
}

fn query_proposal_id(deps: Deps, id: u64) -> StdResult<Proposal> {
    PROPOSALS.load(deps.storage, id.into())
}

fn query_all_proposals(deps: Deps) -> StdResult<AllProposalsResponse> {
    let all: StdResult<Vec<(Vec<u8>, Proposal)>> = PROPOSALS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    all.map(|p| {
        let res = p.into_iter().map(|x| x.1).collect();

        AllProposalsResponse { proposals: res }
    })
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, init, query_all_proposals, query_proposal_id};
    use crate::error::ContractError;
    use crate::matching::QuadraticFundingAlgorithm;
    use crate::msg::{AllProposalsResponse, ExecuteMsg, InitMsg};
    use crate::state::{Proposal, PROPOSALS};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, BankMsg, Binary, CosmosMsg, SubMsg};
    use cw0::Expiration;

    #[test]
    fn create_proposal() {
        let mut env = mock_env();
        let info = mock_info("addr", &[coin(1000, "ucosm")]);
        let mut deps = mock_dependencies(&[]);

        let init_msg = InitMsg {
            admin: "addr".to_string(),
            leftover_addr: "addr".to_string(),
            create_proposal_whitelist: None,
            vote_proposal_whitelist: None,
            voting_period: Expiration::AtHeight(env.block.height + 15),
            proposal_period: Expiration::AtHeight(env.block.height + 10),
            budget_denom: String::from("ucosm"),
            algorithm: QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism {
                parameter: "".to_string(),
            },
        };

        init(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();
        let msg = ExecuteMsg::CreateProposal {
            title: String::from("test"),
            description: String::from("test"),
            metadata: Some(b"test".into()),
            fund_address: "fund_address".to_string(),
        };

        execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        // proposal period expired
        env.block.height += 1000;
        let res = execute(deps.as_mut(), env, info, msg.clone());
        match res {
            Ok(_) => panic!("expected error"),
            Err(ContractError::ProposalPeriodExpired {}) => {}
            e => panic!("unexpected error, got {:?}", e),
        }

        // unauthorised
        let env = mock_env();
        let info = mock_info("true", &[coin(1000, "ucosm")]);
        let mut deps = mock_dependencies(&[]);
        let init_msg = InitMsg {
            leftover_addr: "addr".to_string(),
            admin: "person".to_string(),
            create_proposal_whitelist: Some(vec!["false".to_string()]),
            vote_proposal_whitelist: None,
            voting_period: Default::default(),
            proposal_period: Default::default(),
            budget_denom: String::from("ucosm"),
            algorithm: QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism {
                parameter: "".to_string(),
            },
        };
        init(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        let res = execute(deps.as_mut(), env, info, msg);
        match res {
            Ok(_) => panic!("expected error"),
            Err(ContractError::Unauthorized {}) => {}
            e => panic!("unexpected error, got {:?}", e),
        }
    }

    #[test]
    fn vote_proposal() {
        let mut env = mock_env();
        let info = mock_info("addr", &[coin(1000, "ucosm")]);
        let mut deps = mock_dependencies(&[]);

        let mut init_msg = InitMsg {
            leftover_addr: "addr".to_string(),
            algorithm: QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism {
                parameter: "".to_string(),
            },
            admin: "addr".to_string(),
            create_proposal_whitelist: None,
            vote_proposal_whitelist: None,
            voting_period: Expiration::AtHeight(env.block.height + 15),
            proposal_period: Expiration::AtHeight(env.block.height + 10),
            budget_denom: String::from("ucosm"),
        };
        init(deps.as_mut(), env.clone(), info.clone(), init_msg.clone()).unwrap();

        let create_proposal_msg = ExecuteMsg::CreateProposal {
            title: String::from("test"),
            description: String::from("test"),
            metadata: Some(Binary::from(b"test")),
            fund_address: "fund_address".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            create_proposal_msg,
        )
        .unwrap();

        let msg = ExecuteMsg::VoteProposal { proposal_id: 1 };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        // double vote prevention
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_) => panic!("expected error"),
            Err(ContractError::AddressAlreadyVotedProject {}) => {}
            e => panic!("unexpected error, got {:?}", e),
        }

        // whitelist check
        let mut deps = mock_dependencies(&[]);
        init_msg.vote_proposal_whitelist = Some(vec!["admin".to_string()]);
        init(deps.as_mut(), env.clone(), info.clone(), init_msg.clone()).unwrap();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_) => panic!("expected error"),
            Err(ContractError::Unauthorized {}) => {}
            e => panic!("unexpected error, got {:?}", e),
        }

        // proposal period expired
        let mut deps = mock_dependencies(&[]);
        init_msg.vote_proposal_whitelist = None;
        init(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();
        env.block.height += 15;
        let res = execute(deps.as_mut(), env, info, msg);

        match res {
            Ok(_) => panic!("expected error"),
            Err(ContractError::VotingPeriodExpired {}) => {}
            e => panic!("unexpected error, got {:?}", e),
        }
    }

    #[test]
    fn trigger_distribution() {
        let env = mock_env();
        let budget = 550000u128;
        let info = mock_info("admin", &[coin(budget, "ucosm")]);
        let mut deps = mock_dependencies(&[]);

        let init_msg = InitMsg {
            leftover_addr: "addr".to_string(),
            algorithm: QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism {
                parameter: "".to_string(),
            },
            admin: "admin".to_string(),
            create_proposal_whitelist: None,
            vote_proposal_whitelist: None,
            voting_period: Expiration::AtHeight(env.block.height + 15),
            proposal_period: Expiration::AtHeight(env.block.height + 10),
            budget_denom: String::from("ucosm"),
        };

        init(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        // insert proposals
        let msg = ExecuteMsg::CreateProposal {
            title: String::from("proposal 1"),
            description: "".to_string(),
            metadata: Some(Binary::from(b"test")),
            fund_address: "fund_address1".to_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreateProposal {
            title: String::from("proposal 2"),
            description: "".to_string(),
            metadata: Some(Binary::from(b"test")),
            fund_address: "fund_address2".to_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreateProposal {
            title: String::from("proposal 3"),
            description: "".to_string(),
            metadata: Some(Binary::from(b"test")),
            fund_address: "fund_address3".to_string(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let msg = ExecuteMsg::CreateProposal {
            title: String::from("proposal 4"),
            description: "".to_string(),
            metadata: Some(Binary::from(b"test")),
            fund_address: "fund_address4".to_string(),
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // insert votes
        // proposal1
        let msg = ExecuteMsg::VoteProposal { proposal_id: 1 };
        let vote11_fund = 1200u128;
        let info = mock_info("address1", &[coin(vote11_fund, "ucosm")]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {:?}", e),
        }

        let vote12_fund = 44999u128;
        let info = mock_info("address2", &[coin(vote12_fund, "ucosm")]);
        execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        let vote13_fund = 33u128;
        let info = mock_info("address3", &[coin(vote13_fund, "ucosm")]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        let proposal1 = vote11_fund + vote12_fund + vote13_fund;

        // proposal2
        let msg = ExecuteMsg::VoteProposal { proposal_id: 2 };

        let vote21_fund = 30000u128;
        let info = mock_info("address4", &[coin(vote21_fund, "ucosm")]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {:?}", e),
        }
        let vote22_fund = 58999u128;
        let info = mock_info("address5", &[coin(vote22_fund, "ucosm")]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        let proposal2 = vote21_fund + vote22_fund;

        // proposal3
        let msg = ExecuteMsg::VoteProposal { proposal_id: 3 };
        let vote31_fund = 230000u128;
        let info = mock_info("address6", &[coin(vote31_fund, "ucosm")]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {:?}", e),
        }
        let vote32_fund = 100u128;
        let info = mock_info("address7", &[coin(vote32_fund, "ucosm")]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        let proposal3 = vote31_fund + vote32_fund;

        // proposal4
        let msg = ExecuteMsg::VoteProposal { proposal_id: 4 };
        let vote41_fund = 100000u128;
        let info = mock_info("address8", &[coin(vote41_fund, "ucosm")]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {:?}", e),
        }
        let vote42_fund = 5u128;
        let info = mock_info("address9", &[coin(vote42_fund, "ucosm")]);
        execute(deps.as_mut(), env, info, msg).unwrap();
        let proposal4 = vote41_fund + vote42_fund;

        let trigger_msg = ExecuteMsg::TriggerDistribution {};
        let info = mock_info("admin", &[]);
        let mut env = mock_env();
        env.block.height += 1000;
        let res = execute(deps.as_mut(), env, info, trigger_msg);

        let expected_msgs: Vec<SubMsg<_>> = vec![
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fund_address1".to_string(),
                amount: vec![coin(106444u128, "ucosm")],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fund_address2".to_string(),
                amount: vec![coin(253601u128, "ucosm")],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fund_address3".to_string(),
                amount: vec![coin(458637u128, "ucosm")],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "fund_address4".to_string(),
                amount: vec![coin(196653u128, "ucosm")],
            })),
            // left over msg
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr".to_string(),
                amount: vec![coin(1u128, "ucosm")],
            })),
        ];
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {:?}", e),
        }

        assert_eq!(expected_msgs, res.unwrap().messages);

        // check total cash in and out
        let expected_msg_total_distr: u128 = expected_msgs
            .into_iter()
            .map(|d| match d.msg {
                CosmosMsg::Bank(BankMsg::Send { amount, .. }) => {
                    amount.iter().map(|c| c.amount.u128()).sum()
                }
                _ => unimplemented!(),
            })
            .collect::<Vec<u128>>()
            .iter()
            .sum();
        let total_fund = proposal1 + proposal2 + proposal3 + proposal4 + budget;

        assert_eq!(total_fund, expected_msg_total_distr)
    }

    #[test]
    fn query_proposal() {
        let mut deps = mock_dependencies(&[]);

        let proposal = Proposal {
            id: 1,
            title: "title".to_string(),
            description: "desc".to_string(),
            metadata: None,
            ..Default::default()
        };

        let err = PROPOSALS.save(&mut deps.storage, 1_u64.into(), &proposal);
        match err {
            Ok(_) => {}
            e => panic!("unexpected error, got {:?}", e),
        }
        let res = query_proposal_id(deps.as_ref(), 1).unwrap();
        assert_eq!(proposal, res);
    }

    #[test]
    fn query_all_proposal() {
        let mut deps = mock_dependencies(&[]);

        let proposal = Proposal {
            id: 1,
            title: "title".to_string(),
            description: "desc".to_string(),
            metadata: None,
            fund_address: Default::default(),
            ..Default::default()
        };
        let _ = PROPOSALS.save(&mut deps.storage, 1_u64.into(), &proposal);

        let proposal1 = Proposal {
            id: 2,
            title: "title 2".to_string(),
            description: "desc".to_string(),
            metadata: None,
            fund_address: Default::default(),
            ..Default::default()
        };
        let _ = PROPOSALS.save(&mut deps.storage, 2_u64.into(), &proposal1);
        let res = query_all_proposals(deps.as_ref()).unwrap();

        assert_eq!(
            AllProposalsResponse {
                proposals: vec![proposal, proposal1]
            },
            res
        );
    }
}
