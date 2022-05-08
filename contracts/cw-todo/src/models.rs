use cosmwasm_std::{StdError, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Status {
    OPEN = 1,
    COMPLETED = 2,
}

impl TryFrom<&u8> for Status {
    type Error = StdError;

    fn try_from(value: &u8) -> StdResult<Self> {
        match value {
            1 => Ok(Status::OPEN),
            2 => Ok(Status::COMPLETED),
            _ => Err(StdError::generic_err("Status must be 1 or 2")),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Todo {
    pub id: u64,
    pub description: String,
    pub status: Status,
}
