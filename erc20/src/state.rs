use named_type::NamedType;
use named_type_derive::NamedType;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm::errors::{contract_err, dyn_contract_err, Result};
use cosmwasm::traits::{ReadonlyStorage, Storage};
use cosmwasm::types::CanonicalAddr;
use cw_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

const PREFIX_BALANCES: &[u8] = b"balances";
const PREFIX_ALLOWANCES: &[u8] = b"allowances";

const KEY_CONSTANTS: &[u8] = b"constants";
const KEY_TOTAL_SUPPLY: &[u8] = b"total_supply";

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, JsonSchema, NamedType)]
pub struct Constants {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Serialize, Debug, Deserialize, Clone, PartialEq, JsonSchema, NamedType)]
/// Source must be a decadic integer >= 0
pub struct Amount(String);

impl Amount {
    pub fn parse(&self) -> Result<u128> {
        match self.0.parse::<u128>() {
            Ok(value) => Ok(value),
            Err(_) => contract_err("Error while parsing string to u128"),
        }
    }

    pub fn validate(&self) -> Result<()> {
        let _ = self.parse()?;
        Ok(())
    }

    pub fn subtract(&self, other: &Amount) -> Result<Amount> {
        let here = self.parse()?;
        let there = other.parse()?;
        if here < there {
            return dyn_contract_err(format!(
                "Insufficient funds: have={}, subtract={}",
                here, there
            ));
        }
        Ok(Amount::from(here - there))
    }

    pub fn add(&self, other: &Amount) -> Result<Amount> {
        let total = self.parse()? + other.parse()?;
        Ok(Amount::from(total))
    }
}

impl Default for Amount {
    fn default() -> Self {
        Amount("0".to_string())
    }
}

impl From<u128> for Amount {
    fn from(val: u128) -> Self {
        Amount(val.to_string())
    }
}

impl From<&str> for Amount {
    fn from(raw: &str) -> Self {
        Amount(raw.to_string())
    }
}

pub fn constants<S: Storage>(storage: &mut S) -> Singleton<S, Constants> {
    singleton(storage, KEY_CONSTANTS)
}

pub fn constants_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, Constants> {
    singleton_read(storage, KEY_CONSTANTS)
}

pub fn total_supply<S: Storage>(storage: &mut S) -> Singleton<S, Amount> {
    singleton(storage, KEY_TOTAL_SUPPLY)
}

pub fn total_supply_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, Amount> {
    singleton_read(storage, KEY_TOTAL_SUPPLY)
}

pub fn balances<S: Storage>(storage: &mut S) -> Bucket<S, Amount> {
    bucket(PREFIX_BALANCES, storage)
}

pub fn balances_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, Amount> {
    bucket_read(PREFIX_BALANCES, storage)
}

pub fn allowances<'a, S: Storage>(
    storage: &'a mut S,
    owner: &CanonicalAddr,
) -> Bucket<'a, S, Amount> {
    Bucket::multilevel(&[PREFIX_ALLOWANCES, owner.as_bytes()], storage)
}

pub fn allowances_read<'a, S: ReadonlyStorage>(
    storage: &'a S,
    owner: &CanonicalAddr,
) -> ReadonlyBucket<'a, S, Amount> {
    ReadonlyBucket::multilevel(&[PREFIX_ALLOWANCES, owner.as_bytes()], storage)
}

#[cfg(test)]
mod tests {
    use super::Amount;
    use cosmwasm::errors::{Error, Result};

    fn parse_u128(val: &str) -> Result<u128> {
        Amount::from(val).parse()
    }

    #[test]
    fn works_for_simple_inputs() {
        assert_eq!(parse_u128("0").expect("could not be parsed"), 0);
        assert_eq!(parse_u128("1").expect("could not be parsed"), 1);
        assert_eq!(parse_u128("345").expect("could not be parsed"), 345);
        assert_eq!(
            parse_u128("340282366920938463463374607431768211455").expect("could not be parsed"),
            340282366920938463463374607431768211455
        );
    }

    #[test]
    fn works_for_leading_zeros() {
        assert_eq!(parse_u128("01").expect("could not be parsed"), 1);
        assert_eq!(parse_u128("001").expect("could not be parsed"), 1);
        assert_eq!(parse_u128("0001").expect("could not be parsed"), 1);
    }

    #[test]
    fn errors_for_empty_input() {
        match parse_u128("") {
            Ok(_) => panic!("must not pass"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Error while parsing string to u128")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn errors_for_values_out_of_range() {
        match parse_u128("-1") {
            Ok(_) => panic!("must not pass"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Error while parsing string to u128")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        match parse_u128("340282366920938463463374607431768211456") {
            Ok(_) => panic!("must not pass"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Error while parsing string to u128")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn fails_for_non_decadic_strings() {
        match parse_u128("0xAB") {
            Ok(_) => panic!("must not pass"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Error while parsing string to u128")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        match parse_u128("0xab") {
            Ok(_) => panic!("must not pass"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Error while parsing string to u128")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        match parse_u128("0b1100") {
            Ok(_) => panic!("must not pass"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Error while parsing string to u128")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }
}
