//! This integration test tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.
//!
//! You can easily convert unit tests to integration tests.
//! 1. First copy them over verbatum,
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
mod mock;
mod storage;
mod querier;
#[macro_use]
extern crate lazy_static;
// these mod above needs move to a lib.rs file.

mod erc20msg;
mod dns_msg;

use cosmwasm_std::{coins, from_binary, HandleResponse, InitResponse, QueryResponse, HumanAddr, Uint128};
use cosmwasm_vm::testing::{init, handle, query};
use crate::mock::{mock_env_addr, install, handler_resp, generate_address};

use crate::dns_msg::{HandleMsg, InitMsg, QueryMsg, GetOwnerResponse};
use crate::erc20msg::{ERC20AllowanceResponse, ERC20InitMsg, InitialBalance, ERC20QueryMsg, Erc20HandleMsg};

fn user_address(index: u8) -> HumanAddr {
    match index {
        0 => HumanAddr("cosmos0000u".to_string()), // contract initializer
        1 => HumanAddr("cosmos1111u".to_string()),
        2 => HumanAddr("cosmos2222u".to_string()),
        3 => HumanAddr("cosmos3333u".to_string()),
        _ => panic!("Unsupported address index"),
    }
}

fn init_msg() -> ERC20InitMsg {
    ERC20InitMsg {
        decimals: 5,
        name: "Wasm token".to_string(),
        symbol: "ETH".to_string(),
        initial_balances: [
            InitialBalance {
                address: user_address(0),
                amount: Uint128::from(888u128),
            },
            InitialBalance {
                address: user_address(1),
                amount: Uint128::from(888u128),
            },
            InitialBalance {
                address: user_address(2),
                amount: Uint128::from(888u128),
            },
            InitialBalance {
                address: user_address(3),
                amount: Uint128::from(888u128),
            },
        ].to_vec(),
    }
}

// This line will test the output of cargo wasm
static DNSWASM: &[u8] = include_bytes!("./dns.wasm");
// You can uncomment this line instead to test productionified build from rust-optimizer
static ERC20WASM: &[u8] = include_bytes!("./erc20.wasm");

#[test]
fn sell_domain() {
    // init erc20 contract
    let erc20_contract_address = generate_address();
    let mut erc20deps = install(erc20_contract_address.clone(), "erc20".to_string(), ERC20WASM.to_vec());

    let init_msg = init_msg();
    let env = mock_env_addr(&erc20deps.api, &user_address(0), &erc20_contract_address.clone(), &[]);
    let res: InitResponse = init(&mut erc20deps, env, init_msg).unwrap();
    assert_eq!(0, res.messages.len());


    // init dns contract
    let dns_contract_address = generate_address();
    let mut dns_deps = install(dns_contract_address.clone(), "dns".to_string(), DNSWASM.to_vec());

    let msg = InitMsg { erc20: erc20_contract_address.clone()};
    let env = mock_env_addr(&dns_deps.api, &user_address(1), &dns_contract_address.clone(), &[]);
    let res: InitResponse = init(&mut dns_deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());


    // register domain
    let env = mock_env_addr(&dns_deps.api, &user_address(2), &dns_contract_address.clone(), &coins(100, "eth"));
    let msg = HandleMsg::RegisterDomain {domain: "www.cosmos.com".to_string() };
    let res: HandleResponse = handle(&mut dns_deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());


    // query domain
    let res: QueryResponse = query(&mut dns_deps, QueryMsg::GetOwner { domain: "www.cosmos.com".to_string() }).unwrap();
    let value: GetOwnerResponse = from_binary(&res).unwrap();
    assert_eq!(user_address(2), value.owner);


    // approve allowance for dns contract to move token to domain raw owner
    let env = mock_env_addr(&erc20deps.api, &user_address(3), &erc20_contract_address.clone(), &coins(100, "eth"));
    let msg = Erc20HandleMsg::Approve {
        spender: dns_contract_address.clone(),
        amount:  Uint128::from(500u128),
    };
    let res: HandleResponse = handle(&mut erc20deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());


    // query allowance
    let res = query(&mut erc20deps, ERC20QueryMsg::Allowance { owner: user_address(3), spender: dns_contract_address.clone() }).unwrap();
    let value: ERC20AllowanceResponse = from_binary(&res).unwrap();
    assert_eq!( Uint128::from(500u128), value.allowance);


    // sell domain: cross-contract query and invoke!
    let env = mock_env_addr(&dns_deps.api, &user_address(2), &dns_contract_address.clone(), &coins(100, "eth"));
    let msg = HandleMsg::SellDomain { buyer: user_address(3), domain: "www.cosmos.com".to_string() };
    let res:HandleResponse = handle(&mut dns_deps, env, msg).unwrap();
    assert_eq!(1, res.messages.len());
    let res:HandleResponse = handler_resp(res, dns_contract_address.clone()).unwrap();
    assert_eq!(0, res.messages.len());


    // query domain
    let res = query(&mut dns_deps, QueryMsg::GetOwner { domain: "www.cosmos.com".to_string() }).unwrap();
    let value: GetOwnerResponse = from_binary(&res).unwrap();
    assert_eq!(user_address(3), value.owner);
}