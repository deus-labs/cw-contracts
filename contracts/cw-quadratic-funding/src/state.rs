use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::matching::QuadraticFundingAlgorithm;
use cosmwasm_std::{Binary, CanonicalAddr, Coin, Storage, Uint128};
use cosmwasm_storage::{singleton, Singleton};
use cw0::Expiration;
use cw_storage_plus::{Item, Map, U64Key};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    // set admin as single address, multisig or contract sig could be used
    pub admin: CanonicalAddr,
    // leftover coins from distribution sent to this address
    pub leftover_addr: CanonicalAddr,
    pub create_proposal_whitelist: Option<Vec<CanonicalAddr>>,
    pub vote_proposal_whitelist: Option<Vec<CanonicalAddr>>,
    pub voting_period: Expiration,
    pub proposal_period: Expiration,
    pub budget: Coin,
    pub algorithm: QuadraticFundingAlgorithm,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, JsonSchema)]
pub struct Proposal {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub metadata: Option<Binary>,
    pub fund_address: CanonicalAddr,
    pub collected_funds: Uint128,
}

pub const PROPOSALS: Map<U64Key, Proposal> = Map::new("proposal");
pub const PROPOSAL_SEQ: &[u8] = b"proposal_seq";

pub fn proposal_seq(storage: &mut dyn Storage) -> Singleton<u64> {
    singleton(storage, PROPOSAL_SEQ)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Vote {
    pub proposal_id: u64,
    pub voter: CanonicalAddr,
    pub fund: Coin,
}

pub const VOTES: Map<(U64Key, &[u8]), Vote> = Map::new("votes");
