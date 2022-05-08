use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::models::Todo;

pub const INDEX: Item<u64> = Item::new("index");

pub const TODOS: Map<(&Addr, u64), Todo> = Map::new("todos");
