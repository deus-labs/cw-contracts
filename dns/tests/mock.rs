use cosmwasm_vm::{Api, Instance, Extern, call_query, call_handle};
use cosmwasm_vm::testing::{mock_env, MockApi, MockInstanceOptions};
use cosmwasm_std::{coins, HandleResponse, WasmQuery,
                   QuerierResult, SystemError, CosmosMsg, WasmMsg, StdResult, HumanAddr, Coin, Env,};
use kv::{Config, Store, Raw};
use std::collections::HashMap;
use std::sync::Mutex;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use crate::storage::MyMockStorage;
use crate::querier::{MyMockQuerier, CallBackFunc};

#[derive(Clone)]
struct ContractInfo {
    name: String,
    code: Vec<u8>,
}

lazy_static! {
    static ref TEST_STORE: Store = Store::new(Config::new("./test").temporary(true)).unwrap();
    static ref CONTRACT_INFO: Mutex<HashMap<HumanAddr, ContractInfo>> = {
        let mut m = HashMap::new();
        Mutex::new(m)
    };
}

pub fn mock_env_addr<A: Api>(api: &A, sender: &HumanAddr, contract_address: &HumanAddr, sent: &[Coin]) -> Env {
    let mut env = mock_env(api, sender, sent);
    env.contract.address = api.canonical_address(contract_address).unwrap();
    env
}

pub fn mock_instance<'a>(
    wasm: &[u8],
    contract_balance: &[Coin],
    contract_address: HumanAddr,
    contract_storage: MyMockStorage<'a>,
    call_back: CallBackFunc,
) -> Instance<MyMockStorage<'a>, MockApi, MyMockQuerier> {
    // check_wasm(wasm, &options.supported_features).unwrap();

    let options = MockInstanceOptions {
        contract_balance: Some(contract_balance),
        ..Default::default()
    };

    // merge balances
    let mut balances = options.balances.to_vec();
    if let Some(contract_balance) = options.contract_balance {
        // Remove old entry if exists
        if let Some(pos) = balances.iter().position(|item| *item.0 == contract_address) {
            balances.remove(pos);
        }
        balances.push((&contract_address, contract_balance));
    }

    let deps = Extern {
        api: MockApi::new(options.canonical_address_length),
        querier: MyMockQuerier::new(&balances, call_back),
        storage: contract_storage,
    };
    Instance::from_code(wasm, deps, options.gas_limit).unwrap()
}

pub fn install<'a>(contract_address: HumanAddr, contract_name: String, contract_code: Vec<u8>) -> Instance<MyMockStorage<'a>, MockApi, MyMockQuerier> {
    let mut contract_bucket = TEST_STORE.bucket::<Raw, Raw>(Some(contract_name.clone().as_str())).unwrap();
    let mut contract_store = MyMockStorage::new(contract_bucket);
    let mut contract_deps = mock_instance(contract_code.clone().as_slice(), &[],
                                          contract_address.clone(), contract_store, query_call_back);

    let mut contract_map = CONTRACT_INFO.lock().unwrap();
    contract_map.insert(contract_address.clone(), ContractInfo{
        name: contract_name,
        code: contract_code
    });
    // you must drop it here, or it will hold the lock and block the test process
    drop(contract_map);

    contract_deps
}

pub fn instantiate<'a>(contract_addr: HumanAddr) -> Instance<MyMockStorage<'a>, MockApi, MyMockQuerier> {
    let mut contract_map = CONTRACT_INFO.lock().unwrap();
    let contract_info = contract_map.get(&contract_addr.clone()).unwrap();

    let contract_bucket = TEST_STORE.bucket::<Raw, Raw>(Some(contract_info.name.as_str())).unwrap();
    let mut contract_store = MyMockStorage::new(contract_bucket);
    let mut contract_deps = mock_instance(contract_info.code.as_slice(), &[],
                                          contract_addr, contract_store, query_call_back);

    // you must drop it here, or it will hold the lock and block the test process
    drop(contract_map);

    contract_deps
}

fn query_call_back(request: &WasmQuery) -> QuerierResult{
    match request{
        WasmQuery::Smart{ contract_addr, msg } => {
            let mut query_deps= instantiate(contract_addr.clone());
            let res = call_query(&mut query_deps, msg.as_slice()).unwrap();
            Ok(res)
        }
        WasmQuery::Raw{ contract_addr, key } => {
            Err(SystemError::NoSuchContract { addr: contract_addr.clone() })
        }
    }
}

pub fn handler_resp(res:HandleResponse, caller: HumanAddr) -> StdResult<HandleResponse>{
    let msgs_itr = res.messages.iter();
    for msg in msgs_itr {
        match msg{
            CosmosMsg::Wasm(wasm_msg) => {
                match wasm_msg{
                    WasmMsg::Execute{ contract_addr, msg, send } => {
                        let mut handler_deps= instantiate(contract_addr.clone());
                        let env = mock_env_addr(&handler_deps.api, &caller, &contract_addr, &coins(100, "okt"));
                        let res = call_handle(&mut handler_deps, &env, msg.as_slice()).unwrap().unwrap();

                        if res.messages.len() > 0 {
                            handler_resp(res, contract_addr.clone());
                        }
                    }
                    _ => {
                    }
                }
            }
            _ => {
            }
        }
    }

    Ok(HandleResponse::default())
}

pub fn generate_address() -> HumanAddr{
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .collect();

    let mut address_prefix = "okchain".to_string();
    address_prefix += &rand_string.to_lowercase();

    HumanAddr(address_prefix)
}