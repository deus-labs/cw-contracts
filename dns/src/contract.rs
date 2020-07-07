use cosmwasm_std::{to_binary, log, Api, WasmMsg, Binary, Env, Extern, HandleResponse, InitResponse,
                   Querier, StdResult, Storage, ReadonlyStorage, HumanAddr, generic_err,
                   CanonicalAddr, Uint128, QueryRequest, WasmQuery};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

use crate::msg::{GetOwnerResponse, HandleMsg, InitMsg, QueryMsg, ActorMsg, QueryErcMsg, AllowanceResponse};

use crate::state::{config, config_read, State};

pub const PREFIX_DOMAIN: &[u8] = b"dns";

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let erc_address = deps.api.canonical_address(&msg.erc20)?;
    let state = State{
        erc: erc_address,
    };

    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::RegisterDomain {domain} => try_register(deps, _env, &domain),
        HandleMsg::SellDomain {buyer, domain} => try_sell(deps, _env, &buyer, &domain),
    }
}

pub fn try_register<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    domain: &String,
) -> StdResult<HandleResponse> {
    // check domain
    let check_result = get_domain(&deps.storage, domain);
    if check_result.is_ok(){
        return Err(generic_err(format!("The domain: {} has already been registered", domain)));
    }

    // set domain owner
    let mut dns_store = PrefixedStorage::new(PREFIX_DOMAIN, &mut deps.storage);
    dns_store.set(domain.as_bytes(), _env.message.sender.as_slice())?;

    Ok(HandleResponse::default())
}

pub fn try_sell<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    buyer: &HumanAddr,
    domain: &String,
) -> StdResult<HandleResponse> {
    // check domain owner
    let check_result = get_domain(&deps.storage, &domain);
    if check_result.is_err() {
        return Err(generic_err(format!("The domain: {} has not been registered", domain)));
    }

    let raw_owner = CanonicalAddr(check_result.unwrap());
    if !raw_owner.eq(&_env.message.sender){
        return Err(generic_err("Permission denied to change other's domain"));
    }

    // check account balance in erc20!
    let erc = config_read(&deps.storage).load()?;
    let erc_address = deps.api.human_address(&erc.erc)?;
    let contract_address = deps.api.human_address(&_env.contract.address)?;
    let request = QueryErcMsg::Allowance {
        owner: buyer.into(),
        spender: contract_address
    };
    let erc_msg = to_binary(&request)?;
    let wasm_query = WasmQuery::Smart{
        contract_addr: erc_address.clone(),
        msg: erc_msg
    };
    let query_msg = QueryRequest::<AllowanceResponse>::Wasm(wasm_query);
    // let query_msg = to_binary(&request)?;
    let res: AllowanceResponse = deps.querier.custom_query(&query_msg)?;
    if res.allowance < Uint128(500) {
        return Err(generic_err(format!(
            "Insufficient allowance: allowance = {}, required = {}",
            res.allowance, 500
        )));
    }

    // change domain owner
    let mut dns_store = PrefixedStorage::new(PREFIX_DOMAIN, &mut deps.storage);
    let new_owner_raw_address = deps.api.canonical_address(&buyer)?;
    dns_store.set(domain.as_bytes(), new_owner_raw_address.as_slice())?;

    // send token to me from buyer in erc20 contract!
    let receiver = deps.api.human_address(&_env.message.sender)?;
    let msg = ActorMsg::TransferFrom {
        owner: buyer.into(),
        recipient: receiver,
        amount: Uint128(500)
    };
    let transfer_msg = to_binary(&msg)?;

    // to_binary(&resp)
    let res = HandleResponse {
        messages: vec![WasmMsg::Execute {
            contract_addr: erc_address,
            msg: transfer_msg,
            send: vec![],
        }.into()],
        log: vec![
            log("action", "sell dns"),
        ],
        data: None,
    };
    Ok(res)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner {domain} => query_domain(deps, &domain),
    }
}

fn query_domain<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    domain: &String,
) -> StdResult<Binary> {
    let result = get_domain(&deps.storage, domain);
    if result.is_err() {
        return Err(generic_err(format!("The domain: {} has not been registered", domain)));
    }

    let resp = GetOwnerResponse{ owner: deps.api.human_address(&CanonicalAddr(result.unwrap())).unwrap()};
    to_binary(&resp)
}

fn get_domain<S: Storage>(store: &S , domain: &String) -> StdResult<Binary> {
    let dns_store = ReadonlyPrefixedStorage::new(PREFIX_DOMAIN, store);
    let result = dns_store.get(domain.as_bytes())?;
    match result {
        Some(data) => Ok(Binary(data)),
        None => Err(generic_err(format!("No record related to domain: {} found!", domain))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg { erc20: HumanAddr("cosmos123".to_string())};
        let env = mock_env(&deps.api, "account1", &coins(1000, "eth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn register_domain() {
        let mut deps = mock_dependencies(20, &coins(2, "eth"));

        let msg = InitMsg { erc20: HumanAddr("cosmos123".to_string())};
        let env = mock_env(&deps.api, "account1", &coins(2, "eth"));

        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // register domain
        let env = mock_env(&deps.api, "account1", &coins(2, "eth"));
        let msg = HandleMsg::RegisterDomain {domain: "www.cosmos.com".to_string() };
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // query domain
        let res = query(&deps, QueryMsg::GetOwner { domain: "www.cosmos.com".to_string() }).unwrap();
        let value: GetOwnerResponse = from_binary(&res).unwrap();
        assert_ne!(HumanAddr("account2".to_string()), value.owner);
        assert_eq!(HumanAddr("account1".to_string()), value.owner);
    }

    // #[test]
    // fn sell_domain() {
    //     // move to integration test
    // }
}
