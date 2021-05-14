use cosmwasm_std::{Addr, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

static CONFIG_KEY: &[u8] = b"config";
static POLL_KEY: &[u8] = b"polls";
static BANK_KEY: &[u8] = b"bank";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub denom: String,
    pub owner: Addr,
    pub poll_count: u64,
    pub staked_tokens: Uint128,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenManager {
    pub token_balance: Uint128,             // total staked balance
    pub locked_tokens: Vec<(u64, Uint128)>, //maps poll_id to weight voted
    pub participated_polls: Vec<u64>,       // poll_id
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Voter {
    pub vote: String,
    pub weight: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum PollStatus {
    InProgress,
    Tally,
    Passed,
    Rejected,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn poll(storage: &mut dyn Storage) -> Bucket<Poll> {
    bucket(storage, POLL_KEY)
}

pub fn poll_read(storage: &dyn Storage) -> ReadonlyBucket<Poll> {
    bucket_read(storage, POLL_KEY)
}

pub fn bank(storage: &mut dyn Storage) -> Bucket<TokenManager> {
    bucket(storage, BANK_KEY)
}

pub fn bank_read(storage: &dyn Storage) -> ReadonlyBucket<TokenManager> {
    bucket_read(storage, BANK_KEY)
}
