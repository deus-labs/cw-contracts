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
//! 3. If you access raw storage, wherever you see something like:
//!      deps.storage.get(CONFIG_KEY).0.expect("error getting data").expect("no data stored");
//!    replace it with:
//!      deps.with_storage(|store| {
//!          let data = store.get(CONFIG_KEY).0.expect("error getting data").expect("no data stored");
//!          //...
//!      });
//! 4. Anywhere you see query(&deps, ...) you must replace it with query(&mut deps, ...)

use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{attr, from_slice, Addr, Coin, Env, MessageInfo, Response, Timestamp, Uint128};
use cosmwasm_storage::{to_length_prefixed, to_length_prefixed_nested};
use cosmwasm_vm::testing::{execute, instantiate, mock_env, mock_instance, query};
use cosmwasm_vm::{BackendApi, Storage};

use cw_erc20::contract::{
    bytes_to_u128, KEY_CONSTANTS, KEY_TOTAL_SUPPLY, PREFIX_ALLOWANCES, PREFIX_BALANCES,
    PREFIX_CONFIG,
};
use cw_erc20::{Constants, ExecuteMsg, InitialBalance, InstantiateMsg, QueryMsg};

static WASM: &[u8] = include_bytes!("../target/wasm32-unknown-unknown/release/cw_erc20.wasm");

fn mock_env_height(
    signer: &Addr,
    sent_coins: &[Coin],
    height: u64,
    time: u64,
) -> (Env, MessageInfo) {
    let info = mock_info(signer.as_str(), sent_coins);
    let mut env = mock_env();
    env.block.height = height;
    env.block.time = Timestamp::from_nanos(time);
    (env, info)
}

fn get_constants<S: Storage>(storage: &S) -> Constants {
    let key = [&to_length_prefixed(PREFIX_CONFIG), KEY_CONSTANTS].concat();
    let data = storage
        .get(&key)
        .0
        .expect("error getting data")
        .expect("no config data stored");
    from_slice(&data).expect("invalid data")
}

fn get_total_supply<S: Storage>(storage: &S) -> u128 {
    let key = [&to_length_prefixed(PREFIX_CONFIG), KEY_TOTAL_SUPPLY].concat();
    let data = storage
        .get(&key)
        .0
        .expect("error getting data")
        .expect("no decimals data stored");
    bytes_to_u128(&data).unwrap()
}

fn get_balance<S: Storage, A: BackendApi>(api: &A, storage: &S, address: &Addr) -> u128 {
    let address_key = api
        .canonical_address(address.as_str())
        .0
        .expect("canonical_address failed");
    let key = [
        &to_length_prefixed(&PREFIX_BALANCES),
        address_key.as_slice(),
    ]
    .concat();
    read_u128(storage, &key)
}

fn get_allowance<S: Storage, A: BackendApi>(
    api: &A,
    storage: &S,
    owner: String,
    spender: String,
) -> u128 {
    let owner_raw_address = api
        .canonical_address(owner.as_str())
        .0
        .expect("canonical_address failed");
    let spender_raw_address = api
        .canonical_address(spender.as_str())
        .0
        .expect("canonical_address failed");
    let key = [
        &to_length_prefixed_nested(&[PREFIX_ALLOWANCES, owner_raw_address.as_slice()]),
        spender_raw_address.as_slice(),
    ]
    .concat();
    return read_u128(storage, &key);
}

// Reads 16 byte storage value into u128
// Returns zero if key does not exist. Errors if data found that is not 16 bytes
fn read_u128<S: Storage>(store: &S, key: &[u8]) -> u128 {
    let result = store.get(key).0.unwrap();
    match result {
        Some(data) => bytes_to_u128(&data).unwrap(),
        None => 0u128,
    }
}

fn address(index: u8) -> Addr {
    match index {
        0 => Addr::unchecked("addr0000".to_string()), // contract instantiateializer
        1 => Addr::unchecked("addr1111".to_string()),
        2 => Addr::unchecked("addr4321".to_string()),
        3 => Addr::unchecked("addr5432".to_string()),
        _ => panic!("Unsupported address index"),
    }
}

fn instantiate_msg() -> InstantiateMsg {
    InstantiateMsg {
        decimals: 5,
        name: "Ash token".to_string(),
        symbol: "ASH".to_string(),
        initial_balances: [
            InitialBalance {
                address: address(1),
                amount: Uint128::from(11u128),
            },
            InitialBalance {
                address: address(2),
                amount: Uint128::from(22u128),
            },
            InitialBalance {
                address: address(3),
                amount: Uint128::from(33u128),
            },
        ]
        .to_vec(),
    }
}

#[test]
fn instantiate_works() {
    let mut deps = mock_instance(WASM, &[]);
    let instantiate_msg = instantiate_msg();
    let (env, info) = mock_env_height(&address(0), &[], 876, 0);
    let res: Response = instantiate(&mut deps, env, info, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());

    // query the store directly
    let api = deps.api().clone();
    deps.with_storage(|storage| {
        assert_eq!(
            get_constants(storage),
            Constants {
                name: "Ash token".to_string(),
                symbol: "ASH".to_string(),
                decimals: 5
            }
        );
        assert_eq!(get_total_supply(storage), 66);
        assert_eq!(get_balance(&api, storage, &address(1)), 11);
        assert_eq!(get_balance(&api, storage, &address(2)), 22);
        assert_eq!(get_balance(&api, storage, &address(3)), 33);
        Ok(())
    })
    .unwrap();
}

#[test]
fn transfer_works() {
    let mut deps = mock_instance(WASM, &[]);
    let instantiate_msg = instantiate_msg();
    let (env1, info1) = mock_env_height(&address(0), &[], 876, 0);
    let res: Response = instantiate(&mut deps, env1, info1, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let sender = address(1);
    let recipient = address(2);

    // Before
    let api = deps.api().clone();
    deps.with_storage(|storage| {
        assert_eq!(get_balance(&api, storage, &sender), 11);
        assert_eq!(get_balance(&api, storage, &recipient), 22);
        Ok(())
    })
    .unwrap();

    // Transfer
    let transfer_msg = ExecuteMsg::Transfer {
        recipient: recipient.clone().to_string(),
        amount: Uint128::from(1u128),
    };
    let (env2, info2) = mock_env_height(&sender, &[], 877, 0);
    let transfer_response: Response = execute(&mut deps, env2, info2, transfer_msg).unwrap();
    assert_eq!(transfer_response.messages.len(), 0);
    assert_eq!(
        transfer_response.attributes,
        vec![
            attr("action", "transfer"),
            attr("sender", sender.as_str()),
            attr("recipient", recipient.as_str()),
        ]
    );

    // After
    deps.with_storage(|storage| {
        assert_eq!(get_balance(&api, storage, &sender), 10);
        assert_eq!(get_balance(&api, storage, &recipient), 23);
        Ok(())
    })
    .unwrap();
}

#[test]
fn approve_works() {
    let mut deps = mock_instance(WASM, &[]);
    let instantiate_msg = instantiate_msg();
    let (env1, info1) = mock_env_height(&address(0), &[], 876, 0);
    let res: Response = instantiate(&mut deps, env1, info1, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let owner = address(1);
    let spender = address(2);

    // Before
    let api = deps.api().clone();
    deps.with_storage(|storage| {
        assert_eq!(
            get_allowance(&api, storage, owner.to_string(), spender.to_string()),
            0
        );
        Ok(())
    })
    .unwrap();

    // Approve
    let approve_msg = ExecuteMsg::Approve {
        spender: spender.clone().to_string(),
        amount: Uint128::from(42u128),
    };
    let (env2, info2) = mock_env_height(&owner, &[], 877, 0);
    let approve_response: Response = execute(&mut deps, env2, info2, approve_msg).unwrap();
    assert_eq!(approve_response.messages.len(), 0);
    assert_eq!(
        approve_response.attributes,
        vec![
            attr("action", "approve"),
            attr("owner", &owner),
            attr("spender", &spender),
        ]
    );

    // After
    deps.with_storage(|storage| {
        assert_eq!(
            get_allowance(&api, storage, owner.to_string(), spender.to_string()),
            42
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn transfer_from_works() {
    let mut deps = mock_instance(WASM, &[]);
    let instantiate_msg = instantiate_msg();
    let (env1, info1) = mock_env_height(&address(0), &[], 876, 0);
    let res: Response = instantiate(&mut deps, env1, info1, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let owner = address(1);
    let spender = address(2);
    let recipient = address(3);

    // Before
    let api = deps.api().clone();
    deps.with_storage(|storage| {
        assert_eq!(get_balance(&api, storage, &owner), 11);
        assert_eq!(get_balance(&api, storage, &recipient), 33);
        assert_eq!(
            get_allowance(&api, storage, owner.to_string(), spender.to_string()),
            0
        );
        Ok(())
    })
    .unwrap();

    // Approve
    let approve_msg = ExecuteMsg::Approve {
        spender: spender.clone().to_string(),
        amount: Uint128::from(42u128),
    };
    let (env2, info2) = mock_env_height(&owner, &[], 877, 0);
    let approve_response: Response = execute(&mut deps, env2, info2, approve_msg).unwrap();
    assert_eq!(approve_response.messages.len(), 0);
    assert_eq!(
        approve_response.attributes,
        vec![
            attr("action", "approve"),
            attr("owner", &owner),
            attr("spender", &spender),
        ]
    );

    // Transfer from
    let transfer_from_msg = ExecuteMsg::TransferFrom {
        owner: owner.clone().to_string(),
        recipient: recipient.clone().to_string(),
        amount: Uint128::from(2u128),
    };
    let (env3, info3) = mock_env_height(&spender, &[], 878, 0);
    let transfer_from_response: Response =
        execute(&mut deps, env3, info3, transfer_from_msg).unwrap();
    assert_eq!(transfer_from_response.messages.len(), 0);
    assert_eq!(
        transfer_from_response.attributes,
        vec![
            attr("action", "transfer_from"),
            attr("spender", &spender),
            attr("sender", &owner),
            attr("recipient", &recipient),
        ]
    );

    // After
    deps.with_storage(|storage| {
        assert_eq!(get_balance(&api, storage, &owner), 9);
        assert_eq!(get_balance(&api, storage, &recipient), 35);
        assert_eq!(
            get_allowance(&api, storage, owner.to_string(), spender.to_string()),
            40
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn burn_works() {
    let mut deps = mock_instance(WASM, &[]);
    let instantiate_msg = instantiate_msg();
    let (env1, info1) = mock_env_height(&address(0), &[], 876, 0);
    let res: Response = instantiate(&mut deps, env1, info1, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let owner = address(1);

    // Before
    let api = deps.api().clone();
    deps.with_storage(|storage| {
        assert_eq!(get_balance(&api, storage, &owner), 11);
        Ok(())
    })
    .unwrap();

    // Burn
    let burn_msg = ExecuteMsg::Burn {
        amount: Uint128::from(1u128),
    };
    let (env2, info2) = mock_env_height(&owner, &[], 877, 0);
    let burn_response: Response = execute(&mut deps, env2, info2, burn_msg).unwrap();
    assert_eq!(burn_response.messages.len(), 0);
    assert_eq!(
        burn_response.attributes,
        vec![
            attr("action", "burn"),
            attr("account", &owner),
            attr("amount", "1")
        ]
    );

    // After
    deps.with_storage(|storage| {
        assert_eq!(get_balance(&api, storage, &owner), 10);
        Ok(())
    })
    .unwrap();
}

#[test]
fn can_query_balance_of_existing_address() {
    let mut deps = mock_instance(WASM, &[]);
    let instantiate_msg = instantiate_msg();
    let (env1, info1) = mock_env_height(&address(0), &[], 450, 550);
    let res: Response = instantiate(&mut deps, env1.clone(), info1, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let query_msg = QueryMsg::Balance {
        address: address(2).to_string(),
    };
    let query_result = query(&mut deps, env1, query_msg).unwrap();
    assert_eq!(query_result.as_slice(), b"{\"balance\":\"22\"}");
}
