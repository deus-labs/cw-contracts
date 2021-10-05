use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128, Storage};
use cw_storage_plus::{Item, Map, U128Key};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub count: i32,
    pub owner: Addr,
}

pub const STATE: Item<State> = Item::new("state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub cw20_addr: Addr
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Pot {
    /// target_addr is the address that will receive the pot
    pub target_addr: Addr,
    /// threshold_amount is the token threshold amount
    pub threshold_amount: Uint128,
    /// collected keeps information on how much is collected for this pot.
    pub collected: Uint128,
    /// ready presents if this pot is ready to be collected.
    pub ready: bool,
}
/// POT_SEQ holds the last pot ID
pub const POT_SEQ: Item<U128Key> = Item::new("pot_seq");
pub const POTS: Map<U128Key, Pot> = Map::new("pot");

