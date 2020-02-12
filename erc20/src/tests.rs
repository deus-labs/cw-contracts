use cosmwasm::errors::Error;
use cosmwasm::mock::{dependencies, mock_params};
use cosmwasm::serde::from_slice;
use cosmwasm::traits::{Api, ReadonlyStorage, Storage};
use cosmwasm::types::{HumanAddr, Params};
use cw_storage::ReadonlyPrefixedStorage;

use crate::contract::{bytes_to_u128, handle, init, query, read_u128};
use crate::msg::{HandleMsg, InitMsg, InitialBalance, QueryMsg};
use crate::state::{
    Amount, Constants, KEY_CONSTANTS, KEY_TOTAL_SUPPLY, PREFIX_ALLOWANCES, PREFIX_BALANCES,
    PREFIX_CONFIG,
};

static CANONICAL_LENGTH: usize = 20;

fn mock_params_height<A: Api>(api: &A, signer: &HumanAddr, height: i64, time: i64) -> Params {
    let mut params = mock_params(api, signer, &[], &[]);
    params.block.height = height;
    params.block.time = time;
    params
}

fn get_constants<S: Storage>(storage: &S) -> Constants {
    let config_storage = ReadonlyPrefixedStorage::new(PREFIX_CONFIG, storage);
    let data = config_storage
        .get(KEY_CONSTANTS)
        .expect("no config data stored");
    from_slice(&data).expect("invalid data")
}

fn get_total_supply<S: Storage>(storage: &S) -> u128 {
    let config_storage = ReadonlyPrefixedStorage::new(PREFIX_CONFIG, storage);
    let data = config_storage
        .get(KEY_TOTAL_SUPPLY)
        .expect("no decimals data stored");
    return bytes_to_u128(&data).unwrap();
}

fn get_balance<S: ReadonlyStorage, A: Api>(api: &A, storage: &S, address: &HumanAddr) -> u128 {
    let address_key = api
        .canonical_address(address)
        .expect("canonical_address failed");
    let balances_storage = ReadonlyPrefixedStorage::new(PREFIX_BALANCES, storage);
    return read_u128(&balances_storage, address_key.as_bytes()).unwrap();
}

fn get_allowance<S: ReadonlyStorage, A: Api>(
    api: &A,
    storage: &S,
    owner: &HumanAddr,
    spender: &HumanAddr,
) -> u128 {
    let owner_raw_address = api
        .canonical_address(owner)
        .expect("canonical_address failed");
    let spender_raw_address = api
        .canonical_address(spender)
        .expect("canonical_address failed");
    let allowances_storage = ReadonlyPrefixedStorage::new(PREFIX_ALLOWANCES, storage);
    let owner_storage =
        ReadonlyPrefixedStorage::new(owner_raw_address.as_bytes(), &allowances_storage);
    return read_u128(&owner_storage, spender_raw_address.as_bytes()).unwrap();
}

mod init {
    use super::*;

    #[test]
    fn works() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: [InitialBalance {
                address: HumanAddr("addr0000".to_string()),
                amount: Amount::from("11223344"),
            }]
            .to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            get_constants(&deps.storage),
            Constants {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9
            }
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            11223344
        );
        assert_eq!(get_total_supply(&deps.storage), 11223344);
    }

    #[test]
    fn works_with_empty_balance() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: [].to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(get_total_supply(&deps.storage), 0);
    }

    #[test]
    fn works_with_multiple_balances() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: [
                InitialBalance {
                    address: HumanAddr("addr0000".to_string()),
                    amount: Amount::from("11"),
                },
                InitialBalance {
                    address: HumanAddr("addr1111".to_string()),
                    amount: Amount::from("22"),
                },
                InitialBalance {
                    address: HumanAddr("addrbbbb".to_string()),
                    amount: Amount::from("33"),
                },
            ]
            .to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            11
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
            22
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
            33
        );
        assert_eq!(get_total_supply(&deps.storage), 66);
    }

    #[test]
    fn works_with_balance_larger_than_53_bit() {
        let mut deps = dependencies(CANONICAL_LENGTH);

        // This value cannot be represented precisely in JavaScript and jq. Both
        //   node -e "console.log(9007199254740993)"
        //   echo '{ "value": 9007199254740993 }' | jq
        // return 9007199254740992
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: [InitialBalance {
                address: HumanAddr("addr0000".to_string()),
                amount: Amount::from("9007199254740993"),
            }]
            .to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            9007199254740993
        );
        assert_eq!(get_total_supply(&deps.storage), 9007199254740993);
    }

    #[test]
    // Typical supply like 100 million tokens with 18 decimals exceeds the 64 bit range
    fn works_with_balance_larger_than_64_bit() {
        let mut deps = dependencies(CANONICAL_LENGTH);

        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: [InitialBalance {
                address: HumanAddr("addr0000".to_string()),
                amount: Amount::from("100000000000000000000000000"),
            }]
            .to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            100000000000000000000000000
        );
        assert_eq!(get_total_supply(&deps.storage), 100000000000000000000000000);
    }

    #[test]
    fn fails_for_balance_larger_than_max_u128() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: [InitialBalance {
                address: HumanAddr("addr0000".to_string()),
                // 2**128 = 340282366920938463463374607431768211456
                amount: Amount::from("340282366920938463463374607431768211456"),
            }]
            .to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let result = init(&mut deps, params, init_msg);
        match result {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Error while parsing string to u128")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn fails_for_large_decimals() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 42,
            initial_balances: [].to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let result = init(&mut deps, params, init_msg);
        match result {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => assert_eq!(msg, "Decimals must not exceed 18"),
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn fails_for_name_too_short() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = InitMsg {
            name: "CC".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: [].to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let result = init(&mut deps, params, init_msg);
        match result {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Name is not in the expected format (3-30 UTF-8 bytes)")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn fails_for_name_too_long() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = InitMsg {
            name: "Cash coin. Cash coin. Cash coin. Cash coin.".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: [].to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let result = init(&mut deps, params, init_msg);
        match result {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Name is not in the expected format (3-30 UTF-8 bytes)")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn fails_for_symbol_too_short() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = InitMsg {
            name: "De De".to_string(),
            symbol: "DD".to_string(),
            decimals: 9,
            initial_balances: [].to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let result = init(&mut deps, params, init_msg);
        match result {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Ticker symbol is not in expected format [A-Z]{3,6}")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn fails_for_symbol_too_long() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = InitMsg {
            name: "Super Coin".to_string(),
            symbol: "SUPERCOIN".to_string(),
            decimals: 9,
            initial_balances: [].to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let result = init(&mut deps, params, init_msg);
        match result {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Ticker symbol is not in expected format [A-Z]{3,6}")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn fails_for_symbol_lowercase() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CaSH".to_string(),
            decimals: 9,
            initial_balances: [].to_vec(),
        };
        let params = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let result = init(&mut deps, params, init_msg);
        match result {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "Ticker symbol is not in expected format [A-Z]{3,6}")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }
}

mod transfer {
    use super::*;

    fn make_init_msg() -> InitMsg {
        InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![
                InitialBalance {
                    address: HumanAddr("addr0000".to_string()),
                    amount: Amount::from("11"),
                },
                InitialBalance {
                    address: HumanAddr("addr1111".to_string()),
                    amount: Amount::from("22"),
                },
                InitialBalance {
                    address: HumanAddr("addrbbbb".to_string()),
                    amount: Amount::from("33"),
                },
            ],
        }
    }

    #[test]
    fn can_send_to_existing_recipient() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Initial state
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            11
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
            22
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
            33
        );
        assert_eq!(get_total_supply(&deps.storage), 66);

        // Transfer
        let transfer_msg = HandleMsg::Transfer {
            recipient: HumanAddr("addr1111".to_string()),
            amount: Amount::from("1"),
        };
        let params2 = mock_params_height(&deps.api, &HumanAddr("addr0000".to_string()), 450, 550);
        let transfer_result = handle(&mut deps, params2, transfer_msg).unwrap();
        assert_eq!(transfer_result.messages.len(), 0);
        assert_eq!(transfer_result.log, Some("transfer successful".to_string()));

        // New state
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            10
        ); // -1
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
            23
        ); // +1
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
            33
        );
        assert_eq!(get_total_supply(&deps.storage), 66);
    }

    #[test]
    fn can_send_to_non_existent_recipient() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Initial state
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            11
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
            22
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
            33
        );
        assert_eq!(get_total_supply(&deps.storage), 66);

        // Transfer
        let transfer_msg = HandleMsg::Transfer {
            recipient: HumanAddr("addr2323".to_string()),
            amount: Amount::from("1"),
        };
        let params2 = mock_params_height(&deps.api, &HumanAddr("addr0000".to_string()), 450, 550);
        let transfer_result = handle(&mut deps, params2, transfer_msg).unwrap();
        assert_eq!(transfer_result.messages.len(), 0);
        assert_eq!(transfer_result.log, Some("transfer successful".to_string()));

        // New state
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            10
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
            22
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr2323".to_string())),
            1
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
            33
        );
        assert_eq!(get_total_supply(&deps.storage), 66);
    }

    #[test]
    fn can_send_zero_amount() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Initial state
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            11
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
            22
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
            33
        );
        assert_eq!(get_total_supply(&deps.storage), 66);

        // Transfer
        let transfer_msg = HandleMsg::Transfer {
            recipient: HumanAddr("addr1111".to_string()),
            amount: Amount::from("0"),
        };
        let params2 = mock_params_height(&deps.api, &HumanAddr("addr0000".to_string()), 450, 550);
        let transfer_result = handle(&mut deps, params2, transfer_msg).unwrap();
        assert_eq!(transfer_result.messages.len(), 0);
        assert_eq!(transfer_result.log, Some("transfer successful".to_string()));

        // New state (unchanged)
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            11
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
            22
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
            33
        );
        assert_eq!(get_total_supply(&deps.storage), 66);
    }

    #[test]
    fn can_send_to_sender() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let sender = HumanAddr("addr0000".to_string());

        // Initial state
        assert_eq!(get_balance(&deps.api, &deps.storage, &sender), 11);

        // Transfer
        let transfer_msg = HandleMsg::Transfer {
            recipient: sender.clone(),
            amount: Amount::from("3"),
        };
        let params2 = mock_params_height(&deps.api, &sender, 450, 550);
        let transfer_result = handle(&mut deps, params2, transfer_msg).unwrap();
        assert_eq!(transfer_result.messages.len(), 0);
        assert_eq!(transfer_result.log, Some("transfer successful".to_string()));

        // New state
        assert_eq!(get_balance(&deps.api, &deps.storage, &sender), 11);
    }

    #[test]
    fn fails_on_insufficient_balance() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Initial state
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            11
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
            22
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
            33
        );
        assert_eq!(get_total_supply(&deps.storage), 66);

        // Transfer
        let transfer_msg = HandleMsg::Transfer {
            recipient: HumanAddr("addr1111".to_string()),
            amount: Amount::from("12"),
        };
        let params2 = mock_params_height(&deps.api, &HumanAddr("addr0000".to_string()), 450, 550);
        let transfer_result = handle(&mut deps, params2, transfer_msg);
        match transfer_result {
            Ok(_) => panic!("expected error"),
            Err(Error::DynContractErr { msg, .. }) => {
                assert_eq!(msg, "Insufficient funds: balance=11, required=12")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // New state (unchanged)
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr0000".to_string())),
            11
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addr1111".to_string())),
            22
        );
        assert_eq!(
            get_balance(&deps.api, &deps.storage, &HumanAddr("addrbbbb".to_string())),
            33
        );
        assert_eq!(get_total_supply(&deps.storage), 66);
    }
}

mod approve {
    use super::*;

    fn make_init_msg() -> InitMsg {
        InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![
                InitialBalance {
                    address: HumanAddr("addr0000".to_string()),
                    amount: Amount::from("11"),
                },
                InitialBalance {
                    address: HumanAddr("addr1111".to_string()),
                    amount: Amount::from("22"),
                },
                InitialBalance {
                    address: HumanAddr("addrbbbb".to_string()),
                    amount: Amount::from("33"),
                },
            ],
        }
    }

    fn make_spender() -> HumanAddr {
        HumanAddr("dadadadadadadada".to_string())
    }

    #[test]
    fn has_zero_allowance_by_default() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Existing owner
        assert_eq!(
            get_allowance(
                &deps.api,
                &deps.storage,
                &HumanAddr("addr0000".to_string()),
                &make_spender()
            ),
            0
        );

        // Non-existing owner
        assert_eq!(
            get_allowance(
                &deps.api,
                &deps.storage,
                &HumanAddr("addr4567".to_string()),
                &make_spender()
            ),
            0
        );
    }

    #[test]
    fn can_set_allowance() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            get_allowance(
                &deps.api,
                &deps.storage,
                &HumanAddr("addr7654".to_string()),
                &make_spender()
            ),
            0
        );

        // First approval
        let owner = HumanAddr("addr7654".to_string());
        let approve_msg1 = HandleMsg::Approve {
            spender: make_spender(),
            amount: Amount::from("334422"),
        };
        let params2 = mock_params_height(&deps.api, &owner, 450, 550);
        let transfer_result = handle(&mut deps, params2, approve_msg1).unwrap();
        assert_eq!(transfer_result.messages.len(), 0);
        assert_eq!(transfer_result.log, Some("approve successful".to_string()));

        assert_eq!(
            get_allowance(&deps.api, &deps.storage, &owner, &make_spender()),
            334422
        );

        // Updated approval
        let approve_msg2 = HandleMsg::Approve {
            spender: make_spender(),
            amount: Amount::from("777888"),
        };
        let params3 = mock_params_height(&deps.api, &owner, 450, 550);
        let transfer_result = handle(&mut deps, params3, approve_msg2).unwrap();
        assert_eq!(transfer_result.messages.len(), 0);
        assert_eq!(transfer_result.log, Some("approve successful".to_string()));

        assert_eq!(
            get_allowance(&deps.api, &deps.storage, &owner, &make_spender()),
            777888
        );
    }
}

mod transfer_from {
    use super::*;

    fn make_init_msg() -> InitMsg {
        InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![
                InitialBalance {
                    address: HumanAddr("addr0000".to_string()),
                    amount: Amount::from("11"),
                },
                InitialBalance {
                    address: HumanAddr("addr1111".to_string()),
                    amount: Amount::from("22"),
                },
                InitialBalance {
                    address: HumanAddr("addrbbbb".to_string()),
                    amount: Amount::from("33"),
                },
            ],
        }
    }

    fn make_spender() -> HumanAddr {
        HumanAddr("dadadadadadadada".to_string())
    }

    #[test]
    fn works() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let owner = HumanAddr("addr0000".to_string());
        let spender = &make_spender();
        let recipient = HumanAddr("addr1212".to_string());

        // Set approval
        let approve_msg = HandleMsg::Approve {
            spender: make_spender(),
            amount: Amount::from("4"),
        };
        let params2 = mock_params_height(&deps.api, &owner, 450, 550);
        let approve_result = handle(&mut deps, params2, approve_msg).unwrap();
        assert_eq!(approve_result.messages.len(), 0);
        assert_eq!(approve_result.log, Some("approve successful".to_string()));

        assert_eq!(get_balance(&deps.api, &deps.storage, &owner), 11);
        assert_eq!(get_allowance(&deps.api, &deps.storage, &owner, spender), 4);

        // Transfer less than allowance but more than balance
        let fransfer_from_msg = HandleMsg::TransferFrom {
            owner: owner.clone(),
            recipient: recipient.clone(),
            amount: Amount::from("3"),
        };
        let params3 = mock_params_height(&deps.api, spender, 450, 550);
        let transfer_result = handle(&mut deps, params3, fransfer_from_msg).unwrap();
        assert_eq!(transfer_result.messages.len(), 0);
        assert_eq!(
            transfer_result.log,
            Some("transfer from successful".to_string())
        );

        // State changed
        assert_eq!(get_balance(&deps.api, &deps.storage, &owner), 8);
        assert_eq!(get_allowance(&deps.api, &deps.storage, &owner, spender), 1);
    }

    #[test]
    fn fails_when_allowance_too_low() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let owner = HumanAddr("addr0000".to_string());
        let spender = &make_spender();
        let recipient = HumanAddr("addr1212".to_string());

        // Set approval
        let approve_msg = HandleMsg::Approve {
            spender: make_spender(),
            amount: Amount::from("2"),
        };
        let params2 = mock_params_height(&deps.api, &owner, 450, 550);
        let approve_result = handle(&mut deps, params2, approve_msg).unwrap();
        assert_eq!(approve_result.messages.len(), 0);
        assert_eq!(approve_result.log, Some("approve successful".to_string()));

        assert_eq!(get_balance(&deps.api, &deps.storage, &owner), 11);
        assert_eq!(get_allowance(&deps.api, &deps.storage, &owner, spender), 2);

        // Transfer less than allowance but more than balance
        let fransfer_from_msg = HandleMsg::TransferFrom {
            owner: owner.clone(),
            recipient: recipient.clone(),
            amount: Amount::from("3"),
        };
        let params3 = mock_params_height(&deps.api, spender, 450, 550);
        let transfer_result = handle(&mut deps, params3, fransfer_from_msg);
        match transfer_result {
            Ok(_) => panic!("expected error"),
            Err(Error::DynContractErr { msg, .. }) => {
                assert_eq!(msg, "Insufficient allowance: allowance=2, required=3")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn fails_when_allowance_is_set_but_balance_too_low() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &HumanAddr("creator".to_string()), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let owner = HumanAddr("addr0000".to_string());
        let spender = &make_spender();
        let recipient = HumanAddr("addr1212".to_string());

        // Set approval
        let approve_msg = HandleMsg::Approve {
            spender: make_spender(),
            amount: Amount::from("20"),
        };
        let params2 = mock_params_height(&deps.api, &owner, 450, 550);
        let approve_result = handle(&mut deps, params2, approve_msg).unwrap();
        assert_eq!(approve_result.messages.len(), 0);
        assert_eq!(approve_result.log, Some("approve successful".to_string()));

        assert_eq!(get_balance(&deps.api, &deps.storage, &owner), 11);
        assert_eq!(get_allowance(&deps.api, &deps.storage, &owner, spender), 20);

        // Transfer less than allowance but more than balance
        let fransfer_from_msg = HandleMsg::TransferFrom {
            owner: owner.clone(),
            recipient: recipient.clone(),
            amount: Amount::from("15"),
        };
        let params3 = mock_params_height(&deps.api, spender, 450, 550);
        let transfer_result = handle(&mut deps, params3, fransfer_from_msg);
        match transfer_result {
            Ok(_) => panic!("expected error"),
            Err(Error::DynContractErr { msg, .. }) => {
                assert_eq!(msg, "Insufficient funds: balance=11, required=15")
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }
}

mod query {
    use super::*;

    fn address(index: u8) -> HumanAddr {
        match index {
            0 => HumanAddr("addr0000".to_string()), // contract initializer
            1 => HumanAddr("addr1111".to_string()),
            2 => HumanAddr("addr4321".to_string()),
            3 => HumanAddr("addr5432".to_string()),
            4 => HumanAddr("addr6543".to_string()),
            _ => panic!("Unsupported address index"),
        }
    }

    fn make_init_msg() -> InitMsg {
        InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![
                InitialBalance {
                    address: address(1),
                    amount: Amount::from("11"),
                },
                InitialBalance {
                    address: address(2),
                    amount: Amount::from("22"),
                },
                InitialBalance {
                    address: address(3),
                    amount: Amount::from("33"),
                },
            ],
        }
    }

    #[test]
    fn can_query_balance_of_existing_address() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &address(0), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let query_msg = QueryMsg::Balance {
            address: address(1),
        };
        let query_result = query(&deps, query_msg).unwrap();
        assert_eq!(query_result, b"{\"balance\":\"11\"}");
    }

    #[test]
    fn can_query_balance_of_nonexisting_address() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &address(0), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let query_msg = QueryMsg::Balance {
            address: address(4), // only indices 1, 2, 3 are initialized
        };
        let query_result = query(&deps, query_msg).unwrap();
        assert_eq!(query_result, b"{\"balance\":\"0\"}");
    }

    #[test]
    fn can_query_allowance_of_existing_addresses() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &address(0), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let owner = address(2);
        let spender = address(1);

        let approve_msg = HandleMsg::Approve {
            spender: spender.clone(),
            amount: Amount::from("42"),
        };
        let params2 = mock_params_height(&deps.api, &owner, 450, 550);
        let transfer_result = handle(&mut deps, params2, approve_msg).unwrap();
        assert_eq!(transfer_result.messages.len(), 0);
        assert_eq!(transfer_result.log, Some("approve successful".to_string()));

        let query_msg = QueryMsg::Allowance {
            owner: owner.clone(),
            spender: spender.clone(),
        };
        let query_result = query(&deps, query_msg).unwrap();
        assert_eq!(query_result, b"{\"allowance\":\"42\"}");
    }

    #[test]
    fn can_query_allowance_of_nonexisting_owner() {
        let mut deps = dependencies(CANONICAL_LENGTH);
        let init_msg = make_init_msg();
        let params1 = mock_params_height(&deps.api, &address(0), 450, 550);
        let res = init(&mut deps, params1, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let owner = address(2);
        let spender = address(1);
        let bob = address(3);

        let approve_msg = HandleMsg::Approve {
            spender: spender.clone(),
            amount: Amount::from("42"),
        };
        let params2 = mock_params_height(&deps.api, &owner, 450, 550);
        let transfer_result = handle(&mut deps, params2, approve_msg).unwrap();
        assert_eq!(transfer_result.messages.len(), 0);
        assert_eq!(transfer_result.log, Some("approve successful".to_string()));

        // different spender
        let query_msg = QueryMsg::Allowance {
            owner: owner.clone(),
            spender: bob.clone(),
        };
        let query_result = query(&deps, query_msg).unwrap();
        assert_eq!(query_result, b"{\"allowance\":\"0\"}");

        // differnet owner
        let query_msg = QueryMsg::Allowance {
            owner: bob.clone(),
            spender: spender.clone(),
        };
        let query_result = query(&deps, query_msg).unwrap();
        assert_eq!(query_result, b"{\"allowance\":\"0\"}");
    }
}
