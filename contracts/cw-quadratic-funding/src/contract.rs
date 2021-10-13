use cosmwasm_std::{
    attr, coin, to_binary, BankMsg, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, Order, StdResult,
};

use crate::error::ContractError;
use crate::helper::extract_budget_coin;
use crate::matching::{calculate_clr, QuadraticFundingAlgorithm, RawGrant};
use crate::msg::{AllProposalsResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{proposal_seq, Config, Proposal, Vote, CONFIG, PROPOSALS, VOTES};
use cosmwasm_storage::nextval;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    msg.validate(env)?;

    let budget = extract_budget_coin(info.sent_funds.as_slice(), &msg.budget_denom)?;
    let mut create_proposal_whitelist: Option<Vec<CanonicalAddr>> = None;
    let mut vote_proposal_whitelist: Option<Vec<CanonicalAddr>> = None;
    if let Some(pwl) = msg.create_proposal_whitelist {
        let mut tmp_wl = vec![];
        for w in pwl {
            tmp_wl.push(deps.api.canonical_address(&w)?)
        }
        create_proposal_whitelist = Some(tmp_wl);
    }
    if let Some(vwl) = msg.vote_proposal_whitelist {
        let mut tmp_wl = vec![];
        for w in vwl {
            tmp_wl.push(deps.api.canonical_address(&w)?)
        }
        vote_proposal_whitelist = Some(tmp_wl);
    }
    let cfg = Config {
        admin: deps.api.canonical_address(&msg.admin)?,
        leftover_addr: deps.api.canonical_address(&msg.leftover_addr)?,
        create_proposal_whitelist,
        vote_proposal_whitelist,
        voting_period: msg.voting_period,
        proposal_period: msg.proposal_period,
        algorithm: msg.algorithm,
        budget,
    };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::CreateProposal {
            title,
            description,
            metadata,
            fund_address,
        } => handle_create_proposal(deps, env, info, title, description, metadata, fund_address),
        HandleMsg::VoteProposal { proposal_id } => {
            handle_vote_proposal(deps, env, info, proposal_id)
        }
        HandleMsg::TriggerDistribution { .. } => handle_trigger_distribution(deps, env, info),
    }
}

pub fn handle_create_proposal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    description: String,
    metadata: Option<Binary>,
    fund_address: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check whitelist
    if let Some(wl) = config.create_proposal_whitelist {
        if !wl.contains(&deps.api.canonical_address(&info.sender)?) {
            return Err(ContractError::Unauthorized {});
        }
    }

    // check proposal expiration
    if config.proposal_period.is_expired(&env.block) {
        return Err(ContractError::ProposalPeriodExpired {});
    }

    let id = nextval(&mut proposal_seq(deps.storage))?;
    let p = Proposal {
        id,
        title: title.clone(),
        description,
        metadata,
        fund_address: deps.api.canonical_address(&fund_address)?,
        ..Default::default()
    };
    PROPOSALS.save(deps.storage, id.into(), &p)?;

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "create_proposal"),
            attr("title", title),
            attr("proposal_id", id),
        ],
        data: Some(Binary::from(id.to_be_bytes())),
    };

    Ok(res)
}

pub fn handle_vote_proposal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<HandleResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check whitelist
    if let Some(wl) = config.vote_proposal_whitelist {
        if !wl.contains(&deps.api.canonical_address(&info.sender)?) {
            return Err(ContractError::Unauthorized {});
        }
    }

    // check voting expiration
    if config.voting_period.is_expired(&env.block) {
        return Err(ContractError::VotingPeriodExpired {});
    }

    // validate sent funds and funding denom matches
    let fund = extract_budget_coin(&info.sent_funds, &config.budget.denom)?;

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
        voter: deps.api.canonical_address(&info.sender)?,
        fund,
    };

    // check sender did not voted on proposal
    let vote_key = VOTES.key((proposal_id.into(), info.sender.as_bytes()));
    if vote_key.may_load(deps.storage)?.is_some() {
        return Err(ContractError::AddressAlreadyVotedProject {});
    }

    // save vote
    vote_key.save(deps.storage, &vote)?;

    let res = HandleResponse {
        attributes: vec![
            attr("action", "vote_proposal"),
            attr("proposal_key", proposal_id),
            attr("voter", deps.api.human_address(&vote.voter)?),
            attr("collected_fund", proposal.collected_funds),
        ],
        ..Default::default()
    };

    Ok(res)
}

pub fn handle_trigger_distribution(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<HandleResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // only admin can trigger distribution
    if deps.api.canonical_address(&info.sender)? != config.admin {
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
            from_address: env.contract.address.clone(),
            to_address: deps.api.human_address(&f.addr)?,
            amount: vec![coin(f.grant + f.collected_vote_funds, &config.budget.denom)],
        }));
    }

    let leftover_msg: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        from_address: env.contract.address,
        to_address: deps.api.human_address(&config.leftover_addr)?,
        amount: vec![coin(leftover, config.budget.denom)],
    });

    msgs.push(leftover_msg);
    let res = HandleResponse {
        messages: msgs,
        attributes: vec![attr("action", "trigger_distribution")],
        data: None,
    };

    Ok(res)
}

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
    use crate::contract::{handle, init, query_all_proposals, query_proposal_id};
    use crate::error::ContractError;
    use crate::matching::QuadraticFundingAlgorithm;
    use crate::msg::{AllProposalsResponse, HandleMsg, InitMsg};
    use crate::state::{Proposal, PROPOSALS};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, BankMsg, Binary, CosmosMsg, HumanAddr};
    use cw0::Expiration;

    #[test]
    fn create_proposal() {
        let mut env = mock_env();
        let info = mock_info("addr", &[coin(1000, "ucosm")]);
        let mut deps = mock_dependencies(&[]);

        let init_msg = InitMsg {
            admin: HumanAddr::from("addr"),
            leftover_addr: HumanAddr::from("addr"),
            create_proposal_whitelist: None,
            vote_proposal_whitelist: None,
            voting_period: Expiration::AtHeight(env.block.height + 15),
            proposal_period: Expiration::AtHeight(env.block.height + 10),
            budget_denom: String::from("ucosm"),
            algorithm: QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism {
                parameter: "".to_string(),
            },
        };

        init(deps.as_mut(), env.clone(), info.clone(), init_msg.clone()).unwrap();
        let msg = HandleMsg::CreateProposal {
            title: String::from("test"),
            description: String::from("test"),
            metadata: Some(b"test".into()),
            fund_address: HumanAddr::from("fund_address"),
        };

        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        // success case
        match res {
            Ok(seq) => assert_eq!(seq.data.unwrap(), Binary::from(1_u64.to_be_bytes())),
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        // proposal period expired
        env.block.height = env.block.height + 1000;
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());

        match res {
            Ok(_) => panic!("expected error"),
            Err(ContractError::ProposalPeriodExpired {}) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        // unauthorised
        let env = mock_env();
        let info = mock_info("true", &[coin(1000, "ucosm")]);
        let mut deps = mock_dependencies(&[]);
        let init_msg = InitMsg {
            leftover_addr: HumanAddr::from("addr"),
            admin: HumanAddr::from("person"),
            create_proposal_whitelist: Some(vec![HumanAddr::from("false")]),
            vote_proposal_whitelist: None,
            voting_period: Default::default(),
            proposal_period: Default::default(),
            budget_denom: String::from("ucosm"),
            algorithm: QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism {
                parameter: "".to_string(),
            },
        };
        init(deps.as_mut(), env.clone(), info.clone(), init_msg.clone()).unwrap();

        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());

        match res {
            Ok(_) => panic!("expected error"),
            Err(ContractError::Unauthorized {}) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }
    }

    #[test]
    fn vote_proposal() {
        let mut env = mock_env();
        let info = mock_info("addr", &[coin(1000, "ucosm")]);
        let mut deps = mock_dependencies(&[]);

        let mut init_msg = InitMsg {
            leftover_addr: HumanAddr::from("addr"),
            algorithm: QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism {
                parameter: "".to_string(),
            },
            admin: HumanAddr::from("addr"),
            create_proposal_whitelist: None,
            vote_proposal_whitelist: None,
            voting_period: Expiration::AtHeight(env.block.height + 15),
            proposal_period: Expiration::AtHeight(env.block.height + 10),
            budget_denom: String::from("ucosm"),
        };
        init(deps.as_mut(), env.clone(), info.clone(), init_msg.clone()).unwrap();

        let create_proposal_msg = HandleMsg::CreateProposal {
            title: String::from("test"),
            description: String::from("test"),
            metadata: Some(Binary::from(b"test")),
            fund_address: HumanAddr::from("fund_address"),
        };

        let res = handle(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            create_proposal_msg.clone(),
        );
        match res {
            Ok(seq) => assert_eq!(seq.data.unwrap(), Binary::from(1_u64.to_be_bytes())),
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        let msg = HandleMsg::VoteProposal { proposal_id: 1 };
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        // success case
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        // double vote prevention
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_) => panic!("expected error"),
            Err(ContractError::AddressAlreadyVotedProject {}) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        // whitelist check
        let mut deps = mock_dependencies(&[]);
        init_msg.vote_proposal_whitelist = Some(vec![HumanAddr::from("admin")]);
        init(deps.as_mut(), env.clone(), info.clone(), init_msg.clone()).unwrap();
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_) => panic!("expected error"),
            Err(ContractError::Unauthorized {}) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        // proposal period expired
        let mut deps = mock_dependencies(&[]);
        init_msg.vote_proposal_whitelist = None;
        init(deps.as_mut(), env.clone(), info.clone(), init_msg.clone()).unwrap();
        env.block.height = env.block.height + 15;
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());

        match res {
            Ok(_) => panic!("expected error"),
            Err(ContractError::VotingPeriodExpired {}) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }
    }

    #[test]
    fn trigger_distribution() {
        let env = mock_env();
        let budget = 550000u128;
        let info = mock_info("admin", &[coin(budget, "ucosm")]);
        let mut deps = mock_dependencies(&[]);

        let init_msg = InitMsg {
            leftover_addr: HumanAddr::from("addr"),
            algorithm: QuadraticFundingAlgorithm::CapitalConstrainedLiberalRadicalism {
                parameter: "".to_string(),
            },
            admin: HumanAddr::from("admin"),
            create_proposal_whitelist: None,
            vote_proposal_whitelist: None,
            voting_period: Expiration::AtHeight(env.block.height + 15),
            proposal_period: Expiration::AtHeight(env.block.height + 10),
            budget_denom: String::from("ucosm"),
        };

        init(deps.as_mut(), env.clone(), info.clone(), init_msg.clone()).unwrap();

        // insert proposals
        let msg = HandleMsg::CreateProposal {
            title: String::from("proposal 1"),
            description: "".to_string(),
            metadata: Some(Binary::from(b"test")),
            fund_address: HumanAddr::from("fund_address1"),
        };
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(seq) => assert_eq!(seq.data.unwrap(), Binary::from(1_u64.to_be_bytes())),
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        let msg = HandleMsg::CreateProposal {
            title: String::from("proposal 2"),
            description: "".to_string(),
            metadata: Some(Binary::from(b"test")),
            fund_address: HumanAddr::from("fund_address2"),
        };
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(seq) => assert_eq!(seq.data.unwrap(), Binary::from(2_u64.to_be_bytes())),
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        let msg = HandleMsg::CreateProposal {
            title: String::from("proposal 3"),
            description: "".to_string(),
            metadata: Some(Binary::from(b"test")),
            fund_address: HumanAddr::from("fund_address3"),
        };
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(seq) => assert_eq!(seq.data.unwrap(), Binary::from(3_u64.to_be_bytes())),
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }
        let msg = HandleMsg::CreateProposal {
            title: String::from("proposal 4"),
            description: "".to_string(),
            metadata: Some(Binary::from(b"test")),
            fund_address: HumanAddr::from("fund_address4"),
        };
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(seq) => assert_eq!(seq.data.unwrap(), Binary::from(4_u64.to_be_bytes())),
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        // insert votes
        // proposal1
        let msg = HandleMsg::VoteProposal { proposal_id: 1 };
        let vote11_fund = 1200u128;
        let info = mock_info("address1", &[coin(vote11_fund, "ucosm")]);
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        let vote12_fund = 44999u128;
        let info = mock_info("address2", &[coin(vote12_fund, "ucosm")]);
        handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let vote13_fund = 33u128;
        let info = mock_info("address3", &[coin(vote13_fund, "ucosm")]);
        handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let proposal1 = vote11_fund + vote12_fund + vote13_fund;

        // proposal2
        let msg = HandleMsg::VoteProposal { proposal_id: 2 };

        let vote21_fund = 30000u128;
        let info = mock_info("address4", &[coin(vote21_fund, "ucosm")]);
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }
        let vote22_fund = 58999u128;
        let info = mock_info("address5", &[coin(vote22_fund, "ucosm")]);
        handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let proposal2 = vote21_fund + vote22_fund;

        // proposal3
        let msg = HandleMsg::VoteProposal { proposal_id: 3 };
        let vote31_fund = 230000u128;
        let info = mock_info("address6", &[coin(vote31_fund, "ucosm")]);
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }
        let vote32_fund = 100u128;
        let info = mock_info("address7", &[coin(vote32_fund, "ucosm")]);
        handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let proposal3 = vote31_fund + vote32_fund;

        // proposal4
        let msg = HandleMsg::VoteProposal { proposal_id: 4 };
        let vote41_fund = 100000u128;
        let info = mock_info("address8", &[coin(vote41_fund, "ucosm")]);
        let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }
        let vote42_fund = 5u128;
        let info = mock_info("address9", &[coin(vote42_fund, "ucosm")]);
        handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let proposal4 = vote41_fund + vote42_fund;

        let trigger_msg = HandleMsg::TriggerDistribution {};
        let info = mock_info("admin", &[]);
        let mut env = mock_env();
        env.block.height += 1000;
        let res = handle(deps.as_mut(), env.clone(), info, trigger_msg);

        let expected_msgs: Vec<CosmosMsg<_>> = vec![
            CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: HumanAddr::from("fund_address1"),
                amount: vec![coin(106444u128, "ucosm")],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: HumanAddr::from("fund_address2"),
                amount: vec![coin(253601u128, "ucosm")],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: HumanAddr::from("fund_address3"),
                amount: vec![coin(458637u128, "ucosm")],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: HumanAddr::from("fund_address4"),
                amount: vec![coin(196653u128, "ucosm")],
            }),
            // left over msg
            CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: HumanAddr::from("addr"),
                amount: vec![coin(1u128, "ucosm")],
            }),
        ];
        match res {
            Ok(_) => {}
            e => panic!("unexpected error, got {}", e.unwrap_err()),
        }

        assert_eq!(expected_msgs, res.unwrap().messages);

        // check total cash in and out
        let expected_msg_total_distr: u128 = expected_msgs
            .into_iter()
            .map(|d| match d {
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
            e => panic!("unexpected error, got {}", e.unwrap_err()),
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
