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

use cosmwasm_std::{
    coins, BlockInfo, Coin, ContractInfo, Env, HumanAddr, InitResponse, MessageInfo,
};
use cosmwasm_storage::to_length_prefixed;
use cosmwasm_vm::testing::{init, mock_info, mock_instance};
use cosmwasm_vm::{from_slice, Api, Storage};

use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cw_escrow::msg::InstantiateMsg;
use cw_escrow::state::State;

// This line will test the output of cargo wasm
static WASM: &[u8] = include_bytes!("../target/wasm32-unknown-unknown/release/cw_escrow.wasm");
// You can uncomment this line instead to test productionified build from rust-optimizer
// static WASM: &[u8] = include_bytes!("../contract.wasm");

fn init_msg_expire_by_height(height: u64) -> InstantiateMsg {
    InstantiateMsg {
        arbiter: HumanAddr::from("verifies"),
        recipient: HumanAddr::from("benefits"),
        end_height: Some(height),
        end_time: None,
    }
}

fn mock_env_info_height(signer: &str, sent: &[Coin], height: u64, time: u64) -> (Env, MessageInfo) {
    let env = Env {
        block: BlockInfo {
            height,
            time,
            ..Default::default()
        },
        contract: ContractInfo {
            address: HumanAddr::from(MOCK_CONTRACT_ADDR),
        },
    };
    let info = mock_info(signer, sent);
    return (env, info);
}

#[test]
fn proper_initialization() {
    let mut deps = mock_instance(WASM, &[]);

    let msg = init_msg_expire_by_height(1000);
    let (env, info) = mock_env_info_height("creator", &coins(1000, "earth"), 876, 0);
    let res: InitResponse = init(&mut deps, env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let api = deps.api().clone();
    deps.with_storage(|store| {
        let config_key_raw = to_length_prefixed(b"config");
        let state: State = from_slice(&store.get(&config_key_raw).0.unwrap().unwrap()).unwrap();
        assert_eq!(
            state,
            State {
                arbiter: api
                    .canonical_address(&HumanAddr::from("verifies"))
                    .0
                    .unwrap(),
                recipient: api
                    .canonical_address(&HumanAddr::from("benefits"))
                    .0
                    .unwrap(),
                source: api
                    .canonical_address(&HumanAddr::from("creator"))
                    .0
                    .unwrap(),
                end_height: Some(1000),
                end_time: None,
            }
        );
        Ok(())
    })
    .unwrap();
}
