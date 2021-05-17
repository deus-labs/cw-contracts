use crate::coin_helpers::validate_sent_sufficient_coin;
use crate::error::ContractError;
use crate::msg::{
    CreatePollResponse, ExecuteMsg, InstantiateMsg, PollResponse, QueryMsg, TokenStakeResponse,
};
use crate::state::{
    bank, bank_read, config, config_read, poll, poll_read, Poll, PollStatus, State, Voter,
};
use cosmwasm_std::{
    attr, coin, entry_point, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Storage, Uint128,
};

pub const VOTING_TOKEN: &str = "voting_token";
pub const DEFAULT_END_HEIGHT_BLOCKS: &u64 = &100_800_u64;
const MIN_STAKE_AMOUNT: u128 = 1;
const MIN_DESC_LENGTH: u64 = 3;
const MAX_DESC_LENGTH: u64 = 64;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        denom: msg.denom,
        owner: info.sender,
        poll_count: 0,
        staked_tokens: Uint128::zero(),
    };

    config(deps.storage).save(&state)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::StakeVotingTokens {} => stake_voting_tokens(deps, env, info),
        ExecuteMsg::WithdrawVotingTokens { amount } => {
            withdraw_voting_tokens(deps, env, info, amount)
        }
        ExecuteMsg::CastVote {
            poll_id,
            vote,
            weight,
        } => cast_vote(deps, env, info, poll_id, vote, weight),
        ExecuteMsg::EndPoll { poll_id } => end_poll(deps, env, info, poll_id),
        ExecuteMsg::CreatePoll {
            quorum_percentage,
            description,
            start_height,
            end_height,
        } => create_poll(
            deps,
            env,
            info,
            quorum_percentage,
            description,
            start_height,
            end_height,
        ),
    }
}

pub fn stake_voting_tokens(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let key = info.sender.as_str().as_bytes();

    let mut token_manager = bank_read(deps.storage).may_load(key)?.unwrap_or_default();

    let mut state = config(deps.storage).load()?;

    validate_sent_sufficient_coin(&info.funds, Some(coin(MIN_STAKE_AMOUNT, &state.denom)))?;
    let funds = info
        .funds
        .iter()
        .find(|coin| coin.denom.eq(&state.denom))
        .unwrap();

    token_manager.token_balance += funds.amount;

    let staked_tokens = state.staked_tokens.u128() + funds.amount.u128();
    state.staked_tokens = Uint128::from(staked_tokens);
    config(deps.storage).save(&state)?;

    bank(deps.storage).save(key, &token_manager)?;

    Ok(Response::default())
}

// Withdraw amount if not staked. By default all funds will be withdrawn.
pub fn withdraw_voting_tokens(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let sender_address_raw = info.sender.as_str().as_bytes();

    if let Some(mut token_manager) = bank_read(deps.storage).may_load(sender_address_raw)? {
        let largest_staked = locked_amount(&sender_address_raw, deps.storage);
        let withdraw_amount = amount.unwrap_or(token_manager.token_balance);
        if largest_staked + withdraw_amount > token_manager.token_balance {
            let max_amount = token_manager.token_balance.checked_sub(largest_staked)?;
            Err(ContractError::ExcessiveWithdraw { max_amount })
        } else {
            let balance = token_manager.token_balance.checked_sub(withdraw_amount)?;
            token_manager.token_balance = balance;

            bank(deps.storage).save(sender_address_raw, &token_manager)?;

            let mut state = config(deps.storage).load()?;
            let staked_tokens = state.staked_tokens.checked_sub(withdraw_amount)?;
            state.staked_tokens = staked_tokens;
            config(deps.storage).save(&state)?;

            Ok(send_tokens(
                &info.sender,
                vec![coin(withdraw_amount.u128(), &state.denom)],
                "approve",
            ))
        }
    } else {
        Err(ContractError::PollNoStake {})
    }
}

/// validate_description returns an error if the description is invalid
fn validate_description(description: &str) -> Result<(), ContractError> {
    if (description.len() as u64) < MIN_DESC_LENGTH {
        Err(ContractError::DescriptionTooShort {
            min_desc_length: MIN_DESC_LENGTH,
        })
    } else if (description.len() as u64) > MAX_DESC_LENGTH {
        Err(ContractError::DescriptionTooLong {
            max_desc_length: MAX_DESC_LENGTH,
        })
    } else {
        Ok(())
    }
}

/// validate_quorum_percentage returns an error if the quorum_percentage is invalid
/// (we require 0-100)
fn validate_quorum_percentage(quorum_percentage: Option<u8>) -> Result<(), ContractError> {
    match quorum_percentage {
        Some(qp) => {
            if qp > 100 {
                return Err(ContractError::PollQuorumPercentageMismatch {
                    quorum_percentage: qp,
                });
            }
            Ok(())
        }
        None => Ok(()),
    }
}

/// validate_end_height returns an error if the poll ends in the past
fn validate_end_height(end_height: Option<u64>, env: Env) -> Result<(), ContractError> {
    if end_height.is_some() && env.block.height >= end_height.unwrap() {
        Err(ContractError::PollCannotEndInPast {})
    } else {
        Ok(())
    }
}

/// create a new poll
pub fn create_poll(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    quorum_percentage: Option<u8>,
    description: String,
    start_height: Option<u64>,
    end_height: Option<u64>,
) -> Result<Response, ContractError> {
    validate_quorum_percentage(quorum_percentage)?;
    validate_end_height(end_height, env.clone())?;
    validate_description(&description)?;

    let mut state = config(deps.storage).load()?;
    let poll_count = state.poll_count;
    let poll_id = poll_count + 1;
    state.poll_count = poll_id;

    let new_poll = Poll {
        creator: info.sender,
        status: PollStatus::InProgress,
        quorum_percentage,
        yes_votes: Uint128::zero(),
        no_votes: Uint128::zero(),
        voters: vec![],
        voter_info: vec![],
        end_height: end_height.unwrap_or(env.block.height + DEFAULT_END_HEIGHT_BLOCKS),
        start_height,
        description,
    };
    let key = state.poll_count.to_be_bytes();
    poll(deps.storage).save(&key, &new_poll)?;

    config(deps.storage).save(&state)?;

    let r = Response {
        submessages: vec![],
        messages: vec![],
        attributes: vec![
            attr("action", "create_poll"),
            attr("creator", new_poll.creator),
            attr("poll_id", &poll_id),
            attr("quorum_percentage", quorum_percentage.unwrap_or(0)),
            attr("end_height", new_poll.end_height),
            attr("start_height", start_height.unwrap_or(0)),
        ],
        data: Some(to_binary(&CreatePollResponse { poll_id })?),
    };
    Ok(r)
}

/*
 * Ends a poll. Only the creator of a given poll can end that poll.
 */
pub fn end_poll(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    poll_id: u64,
) -> Result<Response, ContractError> {
    let key = &poll_id.to_be_bytes();
    let mut a_poll = poll(deps.storage).load(key)?;

    if a_poll.creator != info.sender {
        return Err(ContractError::PollNotCreator {
            creator: a_poll.creator.to_string(),
            sender: info.sender.to_string(),
        });
    }

    if a_poll.status != PollStatus::InProgress {
        return Err(ContractError::PollNotInProgress {});
    }

    if let Some(start_height) = a_poll.start_height {
        if start_height > env.block.height {
            return Err(ContractError::PoolVotingPeriodNotStarted { start_height });
        }
    }

    if a_poll.end_height > env.block.height {
        return Err(ContractError::PollVotingPeriodNotExpired {
            expire_height: a_poll.end_height,
        });
    }

    let mut no = 0u128;
    let mut yes = 0u128;

    for voter in &a_poll.voter_info {
        if voter.vote == "yes" {
            yes += voter.weight.u128();
        } else {
            no += voter.weight.u128();
        }
    }
    let tallied_weight = yes + no;

    let mut rejected_reason = "";
    let mut passed = false;

    if tallied_weight > 0 {
        let state = config_read(deps.storage).load()?;

        let staked_weight = deps
            .querier
            .query_balance(&env.contract.address, &state.denom)
            .unwrap()
            .amount
            .u128();

        if staked_weight == 0 {
            return Err(ContractError::PollNoStake {});
        }

        let quorum = ((tallied_weight / staked_weight) * 100) as u8;
        if a_poll.quorum_percentage.is_some() && quorum < a_poll.quorum_percentage.unwrap() {
            // Quorum: More than quorum_percentage of the total staked tokens at the end of the voting
            // period need to have participated in the vote.
            rejected_reason = "Quorum not reached";
        } else if yes > tallied_weight / 2 {
            //Threshold: More than 50% of the tokens that participated in the vote
            // (after excluding “Abstain” votes) need to have voted in favor of the proposal (“Yes”).
            a_poll.status = PollStatus::Passed;
            passed = true;
        } else {
            rejected_reason = "Threshold not reached";
        }
    } else {
        rejected_reason = "Quorum not reached";
    }
    if !passed {
        a_poll.status = PollStatus::Rejected
    }
    poll(deps.storage).save(key, &a_poll)?;

    for voter in &a_poll.voters {
        unlock_tokens(deps.storage, voter, poll_id)?;
    }

    let attributes = vec![
        attr("action", "end_poll"),
        attr("poll_id", &poll_id),
        attr("rejected_reason", rejected_reason),
        attr("passed", &passed),
    ];

    let r = Response {
        submessages: vec![],
        messages: vec![],
        attributes,
        data: None,
    };
    Ok(r)
}

// unlock voter's tokens in a given poll
fn unlock_tokens(
    storage: &mut dyn Storage,
    voter: &Addr,
    poll_id: u64,
) -> Result<Response, ContractError> {
    let voter_key = &voter.as_str().as_bytes();
    let mut token_manager = bank_read(storage).load(voter_key).unwrap();

    // unlock entails removing the mapped poll_id, retaining the rest
    token_manager.locked_tokens.retain(|(k, _)| k != &poll_id);
    bank(storage).save(voter_key, &token_manager)?;
    Ok(Response::default())
}

// finds the largest locked amount in participated polls.
fn locked_amount(voter: &[u8], storage: &dyn Storage) -> Uint128 {
    let token_manager = bank_read(storage).load(voter).unwrap();
    token_manager
        .locked_tokens
        .iter()
        .map(|(_, v)| *v)
        .max()
        .unwrap_or_default()
}

fn has_voted(voter: &Addr, a_poll: &Poll) -> bool {
    a_poll.voters.iter().any(|i| i == voter)
}

pub fn cast_vote(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    poll_id: u64,
    vote: String,
    weight: Uint128,
) -> Result<Response, ContractError> {
    let poll_key = &poll_id.to_be_bytes();
    let state = config_read(deps.storage).load()?;
    if poll_id == 0 || state.poll_count > poll_id {
        return Err(ContractError::PollNotExist {});
    }

    let mut a_poll = poll(deps.storage).load(poll_key)?;

    if a_poll.status != PollStatus::InProgress {
        return Err(ContractError::PollNotInProgress {});
    }

    if has_voted(&info.sender, &a_poll) {
        return Err(ContractError::PollSenderVoted {});
    }

    let key = info.sender.as_str().as_bytes();
    let mut token_manager = bank_read(deps.storage).may_load(key)?.unwrap_or_default();

    if token_manager.token_balance < weight {
        return Err(ContractError::PollInsufficientStake {});
    }
    token_manager.participated_polls.push(poll_id);
    token_manager.locked_tokens.push((poll_id, weight));
    bank(deps.storage).save(key, &token_manager)?;

    a_poll.voters.push(info.sender.clone());

    let voter_info = Voter { vote, weight };

    a_poll.voter_info.push(voter_info);
    poll(deps.storage).save(poll_key, &a_poll)?;

    let attributes = vec![
        attr("action", "vote_casted"),
        attr("poll_id", &poll_id),
        attr("weight", &weight),
        attr("voter", &info.sender),
    ];

    let r = Response {
        submessages: vec![],
        messages: vec![],
        attributes,
        data: None,
    };
    Ok(r)
}

fn send_tokens(to_address: &Addr, amount: Vec<Coin>, action: &str) -> Response {
    let attributes = vec![attr("action", action), attr("to", to_address.clone())];

    Response {
        submessages: vec![],
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            to_address: to_address.to_string(),
            amount,
        })],
        attributes,
        data: None,
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&config_read(deps.storage).load()?),
        QueryMsg::TokenStake { address } => {
            token_balance(deps, deps.api.addr_validate(address.as_str())?)
        }
        QueryMsg::Poll { poll_id } => query_poll(deps, poll_id),
    }
}

fn query_poll(deps: Deps, poll_id: u64) -> StdResult<Binary> {
    let key = &poll_id.to_be_bytes();

    let poll = match poll_read(deps.storage).may_load(key)? {
        Some(poll) => Some(poll),
        None => return Err(StdError::generic_err("Poll does not exist")),
    }
    .unwrap();

    let resp = PollResponse {
        creator: poll.creator.to_string(),
        status: poll.status,
        quorum_percentage: poll.quorum_percentage,
        end_height: Some(poll.end_height),
        start_height: poll.start_height,
        description: poll.description,
    };
    to_binary(&resp)
}

fn token_balance(deps: Deps, address: Addr) -> StdResult<Binary> {
    let token_manager = bank_read(deps.storage)
        .may_load(address.as_str().as_bytes())?
        .unwrap_or_default();

    let resp = TokenStakeResponse {
        token_balance: token_manager.token_balance,
    };

    to_binary(&resp)
}
