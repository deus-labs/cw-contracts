use crate::matching::QuadraticFundingAlgorithm;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Coin, Uint128};
use cw0::Expiration;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    // set admin as single address, multisig or contract sig could be used
    pub admin: String,
    // leftover coins from distribution sent to this address
    pub leftover_addr: String,
    pub create_proposal_whitelist: Option<Vec<String>>,
    pub vote_proposal_whitelist: Option<Vec<String>>,
    pub voting_period: Expiration,
    pub proposal_period: Expiration,
    pub budget: Coin,
    pub algorithm: QuadraticFundingAlgorithm,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
#[derive(Default)]
pub struct Proposal {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub metadata: Option<Binary>,
    pub fund_address: String,
    pub collected_funds: Uint128,
}

pub const PROPOSALS: Map<u64, Proposal> = Map::new("proposal");
pub const PROPOSAL_SEQ: Item<u64> = Item::new("proposal_seq");

#[cw_serde]
pub struct Vote {
    pub proposal_id: u64,
    pub voter: String,
    pub fund: Coin,
}

pub const VOTES: Map<(u64, &[u8]), Vote> = Map::new("votes");
