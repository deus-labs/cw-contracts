use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub arbiter: Addr,
    pub recipient: Addr,
    pub source: Addr,
    pub expiration: Option<Expiration>,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
