use crate::state::State;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;

#[cw_serde]
pub struct InstantiateMsg {
    // owner and creator come from env
    // collateral comes from env
    pub counter_offer: Vec<Coin>,
    pub expires: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Owner can transfer to a new owner
    Transfer { recipient: String },
    /// Owner can post counter_offer on unexpired option to execute and get the collateral
    Execute {},
    /// Burn will release collateral if expired
    Burn {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
}

// We define a custom struct for each query response
pub type ConfigResponse = State;
