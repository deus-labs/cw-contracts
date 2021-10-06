#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, PotResponse, QueryMsg, ReceiveMsg};
use crate::state::{save_pot, Config, Pot, CONFIG, POTS, POT_SEQ};
use cw20::{Cw20Contract, Cw20ExecuteMsg, Cw20ReceiveMsg};

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
        ExecuteMsg::Receive(msg) => execute_receive(deps, info, msg),
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
    };
    save_pot(deps, &pot)?;

    Ok(Response::new()
        .add_attribute("action", "execute_create_pot")
        .add_attribute("target_addr", target_addr)
        .add_attribute("threshold_amount", threshold))
}

pub fn execute_receive(
    deps: DepsMut,
    info: MessageInfo,
    wrapped: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // cw20 address authentication
    let config = CONFIG.load(deps.storage)?;
    if config.cw20_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let msg: ReceiveMsg = from_binary(&wrapped.msg)?;
    match msg {
        ReceiveMsg::Send { id } => receive_send(deps, id, wrapped.amount, info.sender),
    }
}

pub fn receive_send(
    deps: DepsMut,
    pot_id: Uint128,
    amount: Uint128,
    cw20_addr: Addr,
) -> Result<Response, ContractError> {
    // load pot
    let mut pot = POTS.load(deps.storage, pot_id.u128().into())?;

    pot.collected += amount;

    POTS.save(deps.storage, pot_id.u128().into(), &pot)?;

    let mut res = Response::new()
        .add_attribute("action", "receive_send")
        .add_attribute("pot_id", pot_id)
        .add_attribute("collected", pot.collected)
        .add_attribute("threshold", pot.threshold);

    if pot.collected >= pot.threshold {
        // Cw20Contract is a function helper that provides several queries and message builder.
        let cw20 = Cw20Contract(cw20_addr);
        // Build a cw20 transfer send msg, that send collected funds to target address
        let msg = cw20.call(Cw20ExecuteMsg::Transfer {
            recipient: pot.target_addr.into_string(),
            amount: pot.collected,
        })?;
        res = res.add_message(msg);
    }

    Ok(res)
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
        threshold: pot.threshold,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{from_binary, Addr, CosmosMsg, WasmMsg};

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
                threshold: Uint128::new(100)
            }
        );
    }

    #[test]
    fn test_receive_send() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            admin: None,
            cw20_addr: String::from("cw20"),
        };
        let mut info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // should create pot
        let msg = ExecuteMsg::CreatePot {
            target_addr: String::from("Some"),
            threshold: Uint128::new(100),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: String::from("cw20"),
            amount: Uint128::new(55),
            msg: to_binary(&ReceiveMsg::Send {
                id: Uint128::new(1),
            })
            .unwrap(),
        });
        info.sender = Addr::unchecked("cw20");
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
                collected: Uint128::new(55),
                threshold: Uint128::new(100)
            }
        );

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: String::from("cw20"),
            amount: Uint128::new(55),
            msg: to_binary(&ReceiveMsg::Send {
                id: Uint128::new(1),
            })
            .unwrap(),
        });
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let msg = res.messages[0].clone().msg;
        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("cw20"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("Some"),
                    amount: Uint128::new(110)
                })
                .unwrap(),
                funds: vec![]
            })
        );

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
                collected: Uint128::new(110),
                threshold: Uint128::new(100)
            }
        );
    }
}
