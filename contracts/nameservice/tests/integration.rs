//! This integration test tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.
//!
//! You can easily convert unit tests to integration tests as follows:
//! 1. Copy them over verbatim
//! 2. Then change
//!      let mut deps = mock_dependencies(20, &[]);
//!    to
//!      let mut deps = mock_instance(WASM, &[]);
//! 3. If you access raw storage, where ever you see something like:
//!      deps.storage.get(CONFIG_KEY).expect("no data stored");
//!    replace it with:
//!      deps.with_storage(|store| {
//!          let data = store.get(CONFIG_KEY).expect("no data stored");
//!          //...
//!      });
//! 4. Anywhere you see query(&deps, ...) you must replace it with query(&mut deps, ...)

use cosmwasm_std::{coin, coins, from_binary, Coin, Response};
use cosmwasm_storage::to_length_prefixed;
use cosmwasm_vm::testing::{
    execute, instantiate, mock_env, mock_instance, query, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_vm::{from_slice, Instance, Storage};

use cosmwasm_std::testing::mock_info;
use cw_nameservice::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ResolveRecordResponse};
use cw_nameservice::state::{Config, CONFIG_KEY};

// This line will test the output of cargo wasm
static WASM: &[u8] = include_bytes!("../target/wasm32-unknown-unknown/release/cw_nameservice.wasm");
// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] = include_bytes!("../contract.wasm");

fn assert_name_owner(
    mut deps: &mut Instance<MockApi, MockStorage, MockQuerier>,
    name: &str,
    owner: &str,
) {
    let res = query(
        &mut deps,
        mock_env(),
        QueryMsg::ResolveRecord {
            name: name.to_string(),
        },
    )
    .unwrap();

    let value: ResolveRecordResponse = from_binary(&res).unwrap();
    assert_eq!(Some(owner.to_string()), value.address);
}

fn mock_instantiate_with_price(
    mut deps: &mut Instance<MockApi, MockStorage, MockQuerier>,
    purchase_price: Coin,
    transfer_price: Coin,
) {
    let msg = InstantiateMsg {
        purchase_price: Some(purchase_price),
        transfer_price: Some(transfer_price),
    };

    let params = mock_info("creator", &coins(2, "token"));
    // unwrap: contract successfully executes InstantiateMsg
    let _res: Response = instantiate(&mut deps, mock_env(), params, msg).unwrap();
}

fn mock_instantiate_no_price(mut deps: &mut Instance<MockApi, MockStorage, MockQuerier>) {
    let msg = InstantiateMsg {
        purchase_price: None,
        transfer_price: None,
    };

    let params = mock_info("creator", &coins(2, "token"));
    // unwrap: contract successfully executes InstantiateMsg
    let _res: Response = instantiate(&mut deps, mock_env(), params, msg).unwrap();
}

fn mock_alice_registers_name(
    mut deps: &mut Instance<MockApi, MockStorage, MockQuerier>,
    sent: &[Coin],
) {
    // alice can register an available name
    let params = mock_info("alice_key", sent);
    let msg = ExecuteMsg::Register {
        name: "alice".to_string(),
    };
    // unwrap: contract successfully executes Register message
    let _res: Response = execute(&mut deps, mock_env(), params, msg).unwrap();
}

#[test]
fn proper_instantiate_no_fees() {
    let mut deps = mock_instance(WASM, &[]);

    mock_instantiate_no_price(&mut deps);

    deps.with_storage(|storage| {
        let key = to_length_prefixed(CONFIG_KEY);
        let data = storage.get(&key).0.unwrap().unwrap();
        let config_state: Config = from_slice(&data).unwrap();

        assert_eq!(
            config_state,
            Config {
                purchase_price: None,
                transfer_price: None
            }
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn proper_instantiate_with_prices() {
    let mut deps = mock_instance(WASM, &[]);

    mock_instantiate_with_price(&mut deps, coin(3, "token"), coin(4, "token"));

    deps.with_storage(|storage| {
        let key = to_length_prefixed(CONFIG_KEY);
        let data = storage.get(&key).0.unwrap().unwrap();
        let config_state: Config = from_slice(&data).unwrap();

        assert_eq!(
            config_state,
            Config {
                purchase_price: Some(coin(3, "token")),
                transfer_price: Some(coin(4, "token")),
            }
        );

        Ok(())
    })
    .unwrap();
}

#[test]
fn register_available_name_and_query_works_with_prices() {
    let mut deps = mock_instance(WASM, &[]);
    mock_instantiate_with_price(&mut deps, coin(2, "token"), coin(2, "token"));
    mock_alice_registers_name(&mut deps, &coins(2, "token"));

    // anyone can register an available name with more fees than needed
    let params = mock_info("bob_key", &coins(5, "token"));
    let msg = ExecuteMsg::Register {
        name: "bob".to_string(),
    };

    // unwrap: contract successfully executes Register message
    let _res: Response = execute(&mut deps, mock_env(), params, msg).unwrap();

    // querying for name resolves to correct address
    assert_name_owner(&mut deps, "alice", "alice_key");
    assert_name_owner(&mut deps, "bob", "bob_key");
}

#[test]
fn register_available_name_and_query_works() {
    let mut deps = mock_instance(WASM, &[]);
    mock_instantiate_no_price(&mut deps);
    mock_alice_registers_name(&mut deps, &[]);

    // querying for name resolves to correct address
    assert_name_owner(&mut deps, "alice", "alice_key");
}

#[test]
fn returns_empty_on_query_unregistered_name() {
    let mut deps = mock_instance(WASM, &[]);

    mock_instantiate_no_price(&mut deps);

    // querying for unregistered name results in NotFound error
    let res = query(
        &mut deps,
        mock_env(),
        QueryMsg::ResolveRecord {
            name: "alice".to_string(),
        },
    )
    .unwrap();
    let value: ResolveRecordResponse = from_binary(&res).unwrap();
    assert_eq!(None, value.address);
}
