use std::convert::TryInto;

use cosmwasm::errors::{contract_err, dyn_contract_err, Result};
use cosmwasm::traits::{Api, Extern, ReadonlyStorage, Storage};
use cosmwasm::types::{CanonicalAddr, HumanAddr, Params, Response};
use cw_storage::{serialize, PrefixedStorage, ReadonlyPrefixedStorage};

use crate::msg::{AllowanceResponse, BalanceResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    Amount, Constants, KEY_CONSTANTS, KEY_TOTAL_SUPPLY, PREFIX_ALLOWANCES, PREFIX_BALANCES,
    PREFIX_CONFIG,
};

pub fn init<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    _params: Params,
    msg: InitMsg,
) -> Result<Response> {
    let mut total_supply: u128 = 0;
    {
        // Initial balances
        let mut balances_store = PrefixedStorage::new(PREFIX_BALANCES, &mut deps.storage);
        for row in msg.initial_balances {
            let raw_address = deps.api.canonical_address(&row.address)?;
            let amount_raw = row.amount.parse()?;
            balances_store.set(raw_address.as_bytes(), &amount_raw.to_be_bytes());
            total_supply += amount_raw;
        }
    }

    // Check name, symbol, decimals
    if !is_valid_name(&msg.name) {
        return contract_err("Name is not in the expected format (3-30 UTF-8 bytes)");
    }
    if !is_valid_symbol(&msg.symbol) {
        return contract_err("Ticker symbol is not in expected format [A-Z]{3,6}");
    }
    if msg.decimals > 18 {
        return contract_err("Decimals must not exceed 18");
    }

    let mut config_store = PrefixedStorage::new(PREFIX_CONFIG, &mut deps.storage);
    let constants = serialize(&Constants {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
    })?;
    config_store.set(KEY_CONSTANTS, &constants);
    config_store.set(KEY_TOTAL_SUPPLY, &total_supply.to_be_bytes());

    Ok(Response::default())
}

pub fn handle<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    params: Params,
    msg: HandleMsg,
) -> Result<Response> {
    match msg {
        HandleMsg::Approve { spender, amount } => try_approve(deps, params, &spender, &amount),
        HandleMsg::Transfer { recipient, amount } => {
            try_transfer(deps, params, &recipient, &amount)
        }
        HandleMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => try_transfer_from(deps, params, &owner, &recipient, &amount),
    }
}

pub fn query<S: Storage, A: Api>(deps: &Extern<S, A>, msg: QueryMsg) -> Result<Vec<u8>> {
    match msg {
        QueryMsg::Balance { address } => {
            let address_key = deps.api.canonical_address(&address)?;
            let balance = read_balance(&deps.storage, &address_key)?;
            let out = serialize(&BalanceResponse {
                balance: Amount::from(balance),
            })?;
            Ok(out)
        }
        QueryMsg::Allowance { owner, spender } => {
            let owner_key = deps.api.canonical_address(&owner)?;
            let spender_key = deps.api.canonical_address(&spender)?;
            let allowance = read_allowance(&deps.storage, &owner_key, &spender_key)?;
            let out = serialize(&AllowanceResponse {
                allowance: Amount::from(allowance),
            })?;
            Ok(out)
        }
    }
}

fn try_transfer<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    params: Params,
    recipient: &HumanAddr,
    amount: &Amount,
) -> Result<Response> {
    let sender_address_raw = &params.message.signer;
    let recipient_address_raw = deps.api.canonical_address(recipient)?;
    let amount_raw = amount.parse()?;

    perform_transfer(
        &mut deps.storage,
        &sender_address_raw,
        &recipient_address_raw,
        amount_raw,
    )?;

    let res = Response {
        messages: vec![],
        log: Some("transfer successful".to_string()),
        data: None,
    };
    Ok(res)
}

fn try_transfer_from<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    params: Params,
    owner: &HumanAddr,
    recipient: &HumanAddr,
    amount: &Amount,
) -> Result<Response> {
    let spender_address_raw = &params.message.signer;
    let owner_address_raw = deps.api.canonical_address(owner)?;
    let recipient_address_raw = deps.api.canonical_address(recipient)?;
    let amount_raw = amount.parse()?;

    let mut allowance = read_allowance(&deps.storage, &owner_address_raw, &spender_address_raw)?;
    if allowance < amount_raw {
        return dyn_contract_err(format!(
            "Insufficient allowance: allowance={}, required={}",
            allowance, amount_raw
        ));
    }
    allowance -= amount_raw;
    write_allowance(
        &mut deps.storage,
        &owner_address_raw,
        &spender_address_raw,
        allowance,
    );
    perform_transfer(
        &mut deps.storage,
        &owner_address_raw,
        &recipient_address_raw,
        amount_raw,
    )?;

    let res = Response {
        messages: vec![],
        log: Some("transfer from successful".to_string()),
        data: None,
    };
    Ok(res)
}

fn try_approve<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    params: Params,
    spender: &HumanAddr,
    amount: &Amount,
) -> Result<Response> {
    let owner_address_raw = &params.message.signer;
    let spender_address_raw = deps.api.canonical_address(spender)?;
    let amount_raw = amount.parse()?;
    write_allowance(
        &mut deps.storage,
        &owner_address_raw,
        &spender_address_raw,
        amount_raw,
    );
    let res = Response {
        messages: vec![],
        log: Some("approve successful".to_string()),
        data: None,
    };
    Ok(res)
}

fn perform_transfer<T: Storage>(
    store: &mut T,
    from: &CanonicalAddr,
    to: &CanonicalAddr,
    amount: u128,
) -> Result<()> {
    let mut balances_store = PrefixedStorage::new(PREFIX_BALANCES, store);

    let mut from_balance = read_u128(&balances_store, from.as_bytes())?;
    if from_balance < amount {
        return dyn_contract_err(format!(
            "Insufficient funds: balance={}, required={}",
            from_balance, amount
        ));
    }
    from_balance -= amount;
    balances_store.set(from.as_bytes(), &from_balance.to_be_bytes());

    let mut to_balance = read_u128(&balances_store, to.as_bytes())?;
    to_balance += amount;
    balances_store.set(to.as_bytes(), &to_balance.to_be_bytes());

    Ok(())
}

// Converts 16 bytes value into u128
// Errors if data found that is not 16 bytes
pub fn bytes_to_u128(data: &[u8]) -> Result<u128> {
    match data[0..16].try_into() {
        Ok(bytes) => Ok(u128::from_be_bytes(bytes)),
        Err(_) => contract_err("Corrupted data found. 16 byte expected."),
    }
}

// Reads 16 byte storage value into u128
// Returns zero if key does not exist. Errors if data found that is not 16 bytes
pub fn read_u128<S: ReadonlyStorage>(store: &S, key: &[u8]) -> Result<u128> {
    return match store.get(key) {
        Some(data) => bytes_to_u128(&data),
        None => Ok(0u128),
    };
}

fn read_balance<S: Storage>(store: &S, owner: &CanonicalAddr) -> Result<u128> {
    let balance_store = ReadonlyPrefixedStorage::new(PREFIX_BALANCES, store);
    return read_u128(&balance_store, owner.as_bytes());
}

fn read_allowance<S: Storage>(
    store: &S,
    owner: &CanonicalAddr,
    spender: &CanonicalAddr,
) -> Result<u128> {
    let allowances_store = ReadonlyPrefixedStorage::new(PREFIX_ALLOWANCES, store);
    let owner_store = ReadonlyPrefixedStorage::new(owner.as_bytes(), &allowances_store);
    return read_u128(&owner_store, spender.as_bytes());
}

fn write_allowance<S: Storage>(
    store: &mut S,
    owner: &CanonicalAddr,
    spender: &CanonicalAddr,
    amount: u128,
) -> () {
    let mut allowances_store = PrefixedStorage::new(PREFIX_ALLOWANCES, store);
    let mut owner_store = PrefixedStorage::new(owner.as_bytes(), &mut allowances_store);
    owner_store.set(spender.as_bytes(), &amount.to_be_bytes());
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
