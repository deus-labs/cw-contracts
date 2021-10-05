#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, PotResponse, QueryMsg};
use crate::state::{save_pot, Config, Pot, CONFIG, POTS, POT_SEQ};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-example";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = msg
        .admin
        .and_then(|s| deps.api.addr_validate(s.as_str()).ok())
        .unwrap_or(info.sender);
    let config = Config {
        owner: owner.clone(),
        cw20_addr: deps.api.addr_validate(msg.cw20_addr.as_str())?,
    };
    CONFIG.save(deps.storage, &config)?;

    // init pot sequence
    POT_SEQ.save(deps.storage, &Uint128::new(0))?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", owner)
        .add_attribute("cw20_addr", msg.cw20_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreatePot {
            target_addr,
            threshold,
        } => execute_create_pot(deps, info, target_addr, threshold),
    }
}

pub fn execute_create_pot(
    deps: DepsMut,
    info: MessageInfo,
    target_addr: String,
    threshold: Uint128,
) -> Result<Response, ContractError> {
    // owner authentication
    let config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    // create and save pot
    let pot = Pot {
        target_addr: deps.api.addr_validate(target_addr.as_str())?,
        threshold,
        collected: Uint128::zero(),
        ready: false,
    };
    save_pot(deps, &pot)?;

    Ok(Response::new()
        .add_attribute("action", "execute_create_pot")
        .add_attribute("target_addr", target_addr)
        .add_attribute("threshold_amount", threshold))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPot { id } => to_binary(&query_pot(deps, id)?),
    }
}

fn query_pot(deps: Deps, id: Uint128) -> StdResult<PotResponse> {
    let pot = POTS.load(deps.storage, id.u128().into())?;
    Ok(PotResponse {
        target_addr: pot.target_addr.into_string(),
        collected: pot.collected,
        ready: pot.ready,
        threshold: pot.threshold,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{from_binary, Addr};

    /*
    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let info = mock_info("creator", &coins(1000, "earth"));
        let msg = InstantiateMsg { admin: None };

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(17, value.count);
    }

     */

    /*
    #[test]
    fn increment() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg { admin: None };
        let info = mock_info("creator", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::Increment {};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should increase counter by 1
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(18, value.count);
    }


    #[test]
    fn reset() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg { admin: None };
        let info = mock_info("creator", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let unauth_info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::Reset { count: 5 };
        let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let auth_info = mock_info("creator", &[]);
        let msg = ExecuteMsg::Reset { count: 5 };
        let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // should now be 5
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(5, value.count);
    }
     */

    #[test]
    fn create_pot() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            admin: None,
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // should create pot
        let msg = ExecuteMsg::CreatePot {
            target_addr: String::from("Some"),
            threshold: Uint128::new(100),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        // query pot
        let msg = QueryMsg::GetPot {
            id: Uint128::new(1),
        };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();

        let pot: Pot = from_binary(&res).unwrap();
        assert_eq!(
            pot,
            Pot {
                target_addr: Addr::unchecked("Some"),
                collected: Default::default(),
                ready: false,
                threshold: Uint128::new(100)
            }
        );
    }
}
