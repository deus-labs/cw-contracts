use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct State {
    pub denom: String,
    pub owner: Addr,
    pub poll_count: u64,
    pub staked_tokens: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct TokenManager {
    pub token_balance: Uint128,             // total staked balance
    pub locked_tokens: Vec<(u64, Uint128)>, //maps poll_id to weight voted
    pub participated_polls: Vec<u64>,       // poll_id
}

#[cw_serde]
pub struct Voter {
    pub vote: String,
    pub weight: Uint128,
}

#[cw_serde]
pub enum PollStatus {
    InProgress,
    Tally,
    Passed,
    Rejected,
}

#[cw_serde]
pub struct Poll {
    pub creator: Addr,
    pub status: PollStatus,
    pub quorum_percentage: Option<u8>,
    pub yes_votes: Uint128,
    pub no_votes: Uint128,
    pub voters: Vec<Addr>,
    pub voter_info: Vec<Voter>,
    pub end_height: u64,
    pub start_height: Option<u64>,
    pub description: String,
}

pub const CONFIG: Item<State> = Item::new("config");
pub const POLLS: Map<&[u8], Poll> = Map::new("polls");
pub const BANK: Map<&[u8], TokenManager> = Map::new("bank");
