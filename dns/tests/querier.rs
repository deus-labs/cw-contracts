use cosmwasm_std::{HumanAddr, Coin, Querier as stdQuerier, WasmQuery, to_binary, QueryRequest,
                   Never, QuerierResult as stdQuerierResult, SystemError, from_slice, Binary};
use cosmwasm_vm::{Querier, QuerierResult};
use serde::Serialize;
use cosmwasm_std::testing::{BankQuerier, StakingQuerier};

pub type CallBackFunc = fn(&WasmQuery) -> stdQuerierResult;

#[derive(Clone, Default)]
pub struct MyMockQuerier {
    querier: StdMockQuerier,
}

impl MyMockQuerier {
    pub fn new(balances: &[(&HumanAddr, &[Coin])], call_back: CallBackFunc) -> Self {
        MyMockQuerier {
            querier: StdMockQuerier::new(balances,  call_back),
        }
    }

    // set a new balance for the given address and return the old balance
    pub fn update_balance<U: Into<HumanAddr>>(
        &mut self,
        addr: U,
        balance: Vec<Coin>,
    ) -> Option<Vec<Coin>> {
        self.querier.update_balance(addr, balance)
    }

    #[cfg(feature = "staking")]
    pub fn with_staking(
        &mut self,
        denom: &str,
        validators: &[cosmwasm_std::Validator],
        delegations: &[cosmwasm_std::FullDelegation],
    ) {
        self.querier.with_staking(denom, validators, delegations);
    }
}


impl Querier for MyMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let res = self.querier.raw_query(bin_request);
        // We don't use FFI, so FfiResult is always Ok() regardless of error on other levels
        Ok(res)
    }
}

impl MyMockQuerier {
    pub fn handle_query<T: Serialize>(&self, request: &QueryRequest<T>) -> QuerierResult {
        // encode the request, then call raw_query
        let bin = match to_binary(request) {
            Ok(raw) => raw,
            Err(e) => {
                return Ok(Err(SystemError::InvalidRequest {
                    error: format!("Serializing query request: {}", e),
                    request: Binary(b"N/A".to_vec()),
                }));
            }
        };
        self.raw_query(bin.as_slice())
    }
}


#[derive(Clone, Default)]
pub struct StdMockQuerier{
    bank: BankQuerier,
    staking: StakingQuerier,
    wasm: CustomizationWasmQuerier,
}

impl StdMockQuerier {
    pub fn new(balances: &[(&HumanAddr, &[Coin])], call_back: CallBackFunc) -> Self {
        StdMockQuerier {
            bank: BankQuerier::new(balances),
            staking: StakingQuerier::default(),
            wasm: CustomizationWasmQuerier::new(call_back),
        }
    }

    // set a new balance for the given address and return the old balance
    pub fn update_balance<U: Into<HumanAddr>>(
        &mut self,
        addr: U,
        balance: Vec<Coin>,
    ) -> Option<Vec<Coin>> {
        None
    }

    #[cfg(feature = "staking")]
    pub fn with_staking(
        &mut self,
        denom: &str,
        validators: &[crate::query::Validator],
        delegations: &[crate::query::FullDelegation],
    ) {
        self.staking = StakingQuerier::new(denom, validators, delegations);
    }
}

impl stdQuerier for StdMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> stdQuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Never> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl StdMockQuerier {
    pub fn handle_query<T>(&self, request: &QueryRequest<T>) -> stdQuerierResult {
        match &request {
            QueryRequest::Bank(bank_query) => self.bank.query(bank_query),
            QueryRequest::Custom(_) => Err(SystemError::UnsupportedRequest {
                kind: "custom".to_string(),
            }),
            QueryRequest::Staking(staking_query) => self.staking.query(staking_query),
            QueryRequest::Wasm(msg) => self.wasm.query(msg),
        }
    }
}

#[derive(Clone)]
struct CustomizationWasmQuerier {
    call_back: CallBackFunc,
}

impl Default for CustomizationWasmQuerier{
    fn default() -> Self {
        CustomizationWasmQuerier { call_back: default_call_back}
    }
}

pub fn default_call_back(request: &WasmQuery) -> stdQuerierResult {
    Err(SystemError::UnsupportedRequest { kind: "Wasm query".to_string() })
}

impl CustomizationWasmQuerier {
    pub fn new(call_back: CallBackFunc) -> Self {
        CustomizationWasmQuerier {call_back }
    }

    fn query(&self, request: &WasmQuery) -> stdQuerierResult {
        (self.call_back)(request)
    }
}