use cosmwasm_schema::{cw_serde};
use cw_storage_plus::{Item, Map};
use cosmwasm_std::{Addr, Coin};

#[cw_serde]
pub struct Config {
    pub purchase_price: Option<Coin>,
    pub transfer_price: Option<Coin>,
}

#[cw_serde]
pub struct NameRecord {
    pub owner: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const NAME_RESOLVER: Map<&[u8], NameRecord> = Map::new("name_resolver");
