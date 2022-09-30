use cosmwasm_std::Coin;
use cosmwasm_schema::{cw_serde, QueryResponses};
use crate::state::{Config};

#[cw_serde]
pub struct InstantiateMsg {
    pub purchase_price: Option<Coin>,
    pub transfer_price: Option<Coin>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Register { name: String },
    Transfer { name: String, to: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // ResolveAddress returns the current address that the name resolves to
    #[returns(ResolveRecordResponse)]
    ResolveRecord { name: String },
    #[returns(ConfigResponse)]
    Config {},
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ResolveRecordResponse {
    pub address: Option<String>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub purchase_price: Option<Coin>,
    pub transfer_price: Option<Coin>,
}

impl Into<ConfigResponse> for Config{
    fn into(self) -> ConfigResponse {
        ConfigResponse{
            purchase_price: self.purchase_price,
            transfer_price: self.transfer_price,
        }
    }
}
