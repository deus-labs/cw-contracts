use crate::state::{PollStatus, State};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    CastVote {
        poll_id: u64,
        vote: String,
        weight: Uint128,
    },
    StakeVotingTokens {},
    WithdrawVotingTokens {
        amount: Option<Uint128>,
    },
    CreatePoll {
        quorum_percentage: Option<u8>,
        description: String,
        start_height: Option<u64>,
        end_height: Option<u64>,
    },
    EndPoll {
        poll_id: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(State)]
    Config {},
    #[returns(TokenStakeResponse)]
    TokenStake { address: String },
    #[returns(PollResponse)]
    Poll { poll_id: u64 },
}

#[cw_serde]
pub struct PollResponse {
    pub creator: String,
    pub status: PollStatus,
    pub quorum_percentage: Option<u8>,
    pub end_height: Option<u64>,
    pub start_height: Option<u64>,
    pub description: String,
}

#[cw_serde]
pub struct CreatePollResponse {
    pub poll_id: u64,
}

#[cw_serde]
pub struct PollCountResponse {
    pub poll_count: u64,
}

#[cw_serde]
pub struct TokenStakeResponse {
    pub token_balance: Uint128,
}
