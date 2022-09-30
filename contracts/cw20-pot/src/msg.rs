use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Uint64};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    /// cw20_addr is the address of the allowed cw20 token
    pub cw20_addr: String,
}

#[cw_serde]
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

#[cw_serde]
pub enum ReceiveMsg {
    // Send sends token to an id with defined pot
    Send { id: Uint64 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetPot returns pot with given id
    #[returns(PotResponse)]
    GetPot { id: Uint64 },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct PotResponse {
    /// target_addr is the address that will receive the pot
    pub target_addr: String,
    /// threshold is the token threshold amount
    pub threshold: Uint128,
    /// collected keeps information on how much is collected for this pot.
    pub collected: Uint128,
}
