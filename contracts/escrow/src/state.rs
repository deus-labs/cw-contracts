use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use cw_utils::Expiration;
use cosmwasm_schema::{cw_serde};

#[cw_serde]
pub struct Config {
    pub arbiter: Addr,
    pub recipient: Addr,
    pub source: Addr,
    pub expiration: Option<Expiration>,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
