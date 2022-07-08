use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Entry {
    pub id: u64,
    pub description: String,
    pub status: Status,
    pub priority: Priority,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Status {
    ToDo,
    InProgress,
    Done,
    Cancelled,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Priority {
    None,
    Low,
    Medium,
    High,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ENTRY_SEQ: Item<u64> = Item::new("entry_seq");
pub const LIST: Map<u64, Entry> = Map::new("list");
