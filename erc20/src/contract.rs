use cosmwasm::errors::Result;
use cosmwasm::traits::{Api, Extern, Storage};
use cosmwasm::types::{CanonicalAddr, HumanAddr, Params, Response};
use cw_storage::serialize;

use crate::msg::{AllowanceResponse, BalanceResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    allowances, allowances_read, balances, balances_read, constants, total_supply, Amount,
    Constants,
};

pub fn init<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    _params: Params,
    msg: InitMsg,
) -> Result<Response> {
    // ensure the InitMsg is valid, and get the total amount while we are at it
    let total = msg.valid_total()?;

    let mut balances_store = balances(&mut deps.storage);
    for row in msg.initial_balances {
        let raw_address = deps.api.canonical_address(&row.address)?;
        balances_store.save(raw_address.as_bytes(), &row.amount)?;
    }

    constants(&mut deps.storage).save(&Constants {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
    })?;
    total_supply(&mut deps.storage).save(&Amount::from(total))?;
    Ok(Response::default())
}

pub fn handle<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    params: Params,
    msg: HandleMsg,
) -> Result<Response> {
    msg.validate()?;
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
            let balance = balances_read(&deps.storage)
                .may_load(address_key.as_bytes())?
                .unwrap_or_default();
            serialize(&BalanceResponse { balance })
        }
        QueryMsg::Allowance { owner, spender } => {
            let owner_key = deps.api.canonical_address(&owner)?;
            let spender_key = deps.api.canonical_address(&spender)?;
            let allowance = allowances_read(&deps.storage, &owner_key)
                .may_load(spender_key.as_bytes())?
                .unwrap_or_default();
            serialize(&AllowanceResponse { allowance })
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
    amount.validate()?;

    perform_transfer(
        &mut deps.storage,
        &sender_address_raw,
        &recipient_address_raw,
        amount,
    )?;
    Ok(response_with_log("transfer successful"))
}

fn try_transfer_from<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    params: Params,
    owner: &HumanAddr,
    recipient: &HumanAddr,
    amount: &Amount,
) -> Result<Response> {
    let spender_address_raw = params.message.signer.as_bytes();
    let owner_address_raw = deps.api.canonical_address(owner)?;
    let recipient_address_raw = deps.api.canonical_address(recipient)?;

    allowances(&mut deps.storage, &owner_address_raw)
        .update(spender_address_raw, &|current: Amount| {
            current.subtract(amount)
        })?;

    perform_transfer(
        &mut deps.storage,
        &owner_address_raw,
        &recipient_address_raw,
        amount,
    )?;
    Ok(response_with_log("transfer from successful"))
}

fn try_approve<S: Storage, A: Api>(
    deps: &mut Extern<S, A>,
    params: Params,
    spender: &HumanAddr,
    amount: &Amount,
) -> Result<Response> {
    let owner_address_raw = &params.message.signer;
    let spender_address_raw = deps.api.canonical_address(spender)?;
    amount.validate()?;
    allowances(&mut deps.storage, &owner_address_raw)
        .save(spender_address_raw.as_bytes(), amount)?;
    Ok(response_with_log("approve successful"))
}

fn perform_transfer<T: Storage>(
    store: &mut T,
    from: &CanonicalAddr,
    to: &CanonicalAddr,
    amount: &Amount,
) -> Result<()> {
    balances(store).update(from.as_bytes(), &|current| current.subtract(amount))?;
    balances(store).may_update(to.as_bytes(), &|current| {
        let updated = current.unwrap_or_default().add(amount)?;
        Ok(Some(updated))
    })?;
    Ok(())
}

fn response_with_log(msg: &str) -> Response {
    Response {
        messages: vec![],
        log: Some(msg.to_string()),
        data: None,
    }
}
