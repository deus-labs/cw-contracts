use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub creator: Addr,
    pub owner: Addr,
    pub collateral: Vec<Coin>,
    pub counter_offer: Vec<Coin>,
    pub expires: u64,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<State> = Item::new(CONFIG_KEY);
