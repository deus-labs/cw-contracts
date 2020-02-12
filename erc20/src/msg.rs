use named_type::NamedType;
use named_type_derive::NamedType;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm::errors::{contract_err, Result};
use cosmwasm::types::HumanAddr;

use crate::state::Amount;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct InitialBalance {
    pub address: HumanAddr,
    pub amount: Amount,
}

impl InitialBalance {
    pub fn valid_amount(&self) -> Result<u128> {
        // ideally we validate the human address as well
        self.amount.parse()
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<InitialBalance>,
}

impl InitMsg {
    // validate the message and return total amount
    pub fn valid_total(&self) -> Result<u128> {
        // Check name, symbol, decimals
        if !is_valid_name(&self.name) {
            return contract_err("Name is not in the expected format (3-30 UTF-8 bytes)");
        }
        if !is_valid_symbol(&self.symbol) {
            return contract_err("Ticker symbol is not in expected format [A-Z]{3,6}");
        }
        if self.decimals > 18 {
            return contract_err("Decimals must not exceed 18");
        }
        // make sure all balances are valid and get the total
        self.initial_balances
            .iter()
            .fold(Ok(0u128), |acc, bal| Ok(acc? + bal.valid_amount()?))
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    Approve {
        spender: HumanAddr,
        amount: Amount,
    },
    Transfer {
        recipient: HumanAddr,
        amount: Amount,
    },
    TransferFrom {
        owner: HumanAddr,
        recipient: HumanAddr,
        amount: Amount,
    },
}

impl HandleMsg {
    pub fn validate(&self) -> Result<()> {
        match self {
            HandleMsg::Approve { amount, .. } => amount.validate(),
            HandleMsg::Transfer { amount, .. } => amount.validate(),
            HandleMsg::TransferFrom { amount, .. } => amount.validate(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum QueryMsg {
    Balance {
        address: HumanAddr,
    },
    Allowance {
        owner: HumanAddr,
        spender: HumanAddr,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, NamedType)]
pub struct BalanceResponse {
    pub balance: Amount,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, NamedType)]
pub struct AllowanceResponse {
    pub allowance: Amount,
}

fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 30 {
        return false;
    }
    return true;
}

fn is_valid_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < 3 || bytes.len() > 6 {
        return false;
    }

    for byte in bytes.iter() {
        if *byte < 65 || *byte > 90 {
            return false;
        }
    }

    return true;
}
