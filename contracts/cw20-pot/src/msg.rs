use cosmwasm_std::{Uint128, Uint64};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    /// cw20_addr is the address of the allowed cw20 token
    pub cw20_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreatePot {
        /// target_addr will receive tokens when token amount threshold is met.
        target_addr: String,
        /// threshold is the token amount for releasing tokens.
        threshold: Uint128,
    },
    /// Receive forwards received cw20 tokens to an execution logic
    Receive(Cw20ReceiveMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    // Send sends token to an id with defined pot
    Send { id: Uint64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetPot returns pot with given id
    GetPot { id: Uint64 },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PotResponse {
    /// target_addr is the address that will receive the pot
    pub target_addr: String,
    /// threshold is the token threshold amount
    pub threshold: Uint128,
    /// collected keeps information on how much is collected for this pot.
    pub collected: Uint128,
}
