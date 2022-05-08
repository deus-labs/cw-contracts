use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Addr, CosmosMsg, StdResult, Storage, WasmMsg};

use crate::msg::ExecuteMsg;
use crate::state::INDEX;

pub fn get_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = INDEX.may_load(store)?.unwrap_or_default();
    let next_id: u64 = &id + 1;
    INDEX.save(store, &next_id)?;
    Ok(id)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }
}
