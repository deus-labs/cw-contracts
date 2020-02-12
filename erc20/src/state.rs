use named_type::NamedType;
use named_type_derive::NamedType;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

//use cw_storage::{serialize, PrefixedStorage, ReadonlyPrefixedStorage};

pub const PREFIX_CONFIG: &[u8] = b"config";
pub const PREFIX_BALANCES: &[u8] = b"balances";
pub const PREFIX_ALLOWANCES: &[u8] = b"allowances";

pub const KEY_CONSTANTS: &[u8] = b"constants";
pub const KEY_TOTAL_SUPPLY: &[u8] = b"total_supply";

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, JsonSchema, NamedType)]
pub struct Constants {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}
