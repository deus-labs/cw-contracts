use crate::state::Config;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;

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

impl From<Config> for ConfigResponse {
    fn from(config: Config) -> ConfigResponse {
        ConfigResponse {
            purchase_price: config.purchase_price,
            transfer_price: config.transfer_price,
        }
    }
}
