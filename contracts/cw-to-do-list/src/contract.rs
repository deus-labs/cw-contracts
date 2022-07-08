#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use std::ops::Add;

use crate::error::ContractError;
use crate::msg::{EntryResponse, ExecuteMsg, InstantiateMsg, ListResponse, QueryMsg};
use crate::state::{Config, Entry, Priority, Status, CONFIG, ENTRY_SEQ, LIST};

// version info for migration
const CONTRACT_NAME: &str = "crates.io:cw-to-do-list";
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
        .owner
        .and_then(|addr_string| deps.api.addr_validate(addr_string.as_str()).ok())
        .unwrap_or(info.sender);

    let config = Config {
        owner: owner.clone(),
    };
    CONFIG.save(deps.storage, &config)?;

    ENTRY_SEQ.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::NewEntry {
            description,
            priority,
        } => execute_create_new_entry(deps, info, description, priority),
        ExecuteMsg::UpdateEntry {
            id,
            description,
            status,
            priority,
        } => execute_update_entry(deps, info, id, description, status, priority),
        ExecuteMsg::DeleteEntry { id } => execute_delete_entry(deps, info, id),
    }
}

pub fn execute_create_new_entry(
    deps: DepsMut,
    info: MessageInfo,
    description: String,
    priority: Option<Priority>,
) -> Result<Response, ContractError> {
    let owner = CONFIG.load(deps.storage)?.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }
    let id = ENTRY_SEQ.update::<_, cosmwasm_std::StdError>(deps.storage, |id| Ok(id.add(1)))?;
    let new_entry = Entry {
        id,
        description,
        priority: priority.unwrap_or(Priority::None),
        status: Status::ToDo,
    };
    LIST.save(deps.storage, id, &new_entry)?;
    Ok(Response::new()
        .add_attribute("method", "execute_create_new_entry")
        .add_attribute("new_entry_id", id.to_string()))
}

pub fn execute_update_entry(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
    description: Option<String>,
    status: Option<Status>,
    priority: Option<Priority>,
) -> Result<Response, ContractError> {
    let owner = CONFIG.load(deps.storage)?.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    let entry = LIST.load(deps.storage, id)?;
    let updated_entry = Entry {
        id,
        description: description.unwrap_or(entry.description),
        status: status.unwrap_or(entry.status),
        priority: priority.unwrap_or(entry.priority),
    };
    LIST.save(deps.storage, id, &updated_entry)?;
    Ok(Response::new()
        .add_attribute("method", "execute_update_entry")
        .add_attribute("updated_entry_id", id.to_string()))
}

pub fn execute_delete_entry(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let owner = CONFIG.load(deps.storage)?.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    LIST.remove(deps.storage, id);
    Ok(Response::new()
        .add_attribute("method", "execute_delete_entry")
        .add_attribute("deleted_entry_id", id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryEntry { id } => to_binary(&query_entry(deps, id)?),
        QueryMsg::QueryList { start_after, limit } => {
            to_binary(&query_list(deps, start_after, limit)?)
        }
    }
}

fn query_entry(deps: Deps, id: u64) -> StdResult<EntryResponse> {
    let entry = LIST.load(deps.storage, id)?;
    Ok(EntryResponse {
        id: entry.id,
        description: entry.description,
        status: entry.status,
        priority: entry.priority,
    })
}

// Limits for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn query_list(deps: Deps, start_after: Option<u64>, limit: Option<u32>) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);
    let entries: StdResult<Vec<_>> = LIST
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect();

    let result = ListResponse {
        entries: entries?.into_iter().map(|l| l.1).collect(),
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, from_binary, Addr};
    use std::vec::Vec;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        //no owner specified in the instantiation message
        let msg = InstantiateMsg { owner: None };
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let state = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(
            state,
            Config {
                owner: Addr::unchecked("creator".to_string()),
            }
        );
        //specifying an owner address in the instantiation message
        let msg = InstantiateMsg {
            owner: Some("specified_owner".to_string()),
        };

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let state = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(
            state,
            Config {
                owner: Addr::unchecked("specified_owner".to_string()),
            }
        );
    }

    #[test]
    fn create_update_delete_entry() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { owner: None };

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = ExecuteMsg::NewEntry {
            description: "A new entry.".to_string(),
            priority: Some(Priority::Medium),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "execute_create_new_entry"),
                attr("new_entry_id", "1")
            ]
        );
        // Query single entry
        let res = query(deps.as_ref(), env.clone(), QueryMsg::QueryEntry { id: 1 }).unwrap();
        let entry: EntryResponse = from_binary(&res).unwrap();
        assert_eq!(
            EntryResponse {
                id: 1,
                description: "A new entry.".to_string(),
                status: Status::ToDo,
                priority: Priority::Medium
            },
            entry
        );

        let msg = ExecuteMsg::NewEntry {
            description: "Another entry.".to_string(),
            priority: Some(Priority::High),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "execute_create_new_entry"),
                attr("new_entry_id", "2")
            ]
        );

        // Query the list of entries
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::QueryList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
        let list: ListResponse = from_binary(&res).unwrap();
        assert_eq!(
            Vec::from([
                Entry {
                    id: 1,
                    description: "A new entry.".to_string(),
                    status: Status::ToDo,
                    priority: Priority::Medium
                },
                Entry {
                    id: 2,
                    description: "Another entry.".to_string(),
                    status: Status::ToDo,
                    priority: Priority::High
                }
            ]),
            list.entries
        );

        // Update entry
        let message = ExecuteMsg::UpdateEntry {
            id: 1,
            description: Some("Updated entry.".to_string()),
            status: Some(Status::InProgress),
            priority: Some(Priority::Low),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), message).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "execute_update_entry"),
                attr("updated_entry_id", "1")
            ]
        );

        // Query single entry
        let res = query(deps.as_ref(), env.clone(), QueryMsg::QueryEntry { id: 1 }).unwrap();
        let entry: EntryResponse = from_binary(&res).unwrap();
        assert_eq!(
            EntryResponse {
                id: 1,
                description: "Updated entry.".to_string(),
                status: Status::InProgress,
                priority: Priority::Low
            },
            entry
        );

        // Query the list of entries
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::QueryList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
        let list: ListResponse = from_binary(&res).unwrap();
        assert_eq!(
            Vec::from([
                Entry {
                    id: 1,
                    description: "Updated entry.".to_string(),
                    status: Status::InProgress,
                    priority: Priority::Low
                },
                Entry {
                    id: 2,
                    description: "Another entry.".to_string(),
                    status: Status::ToDo,
                    priority: Priority::High
                }
            ]),
            list.entries
        );

        //Delete Entry
        let message = ExecuteMsg::DeleteEntry { id: 1 };

        let res = execute(deps.as_mut(), env.clone(), info, message).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "execute_delete_entry"),
                attr("deleted_entry_id", "1")
            ]
        );
        // Query the list of entries
        let res = query(
            deps.as_ref(),
            env,
            QueryMsg::QueryList {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
        let list: ListResponse = from_binary(&res).unwrap();
        assert_eq!(
            Vec::from([Entry {
                id: 2,
                description: "Another entry.".to_string(),
                status: Status::ToDo,
                priority: Priority::High
            }]),
            list.entries
        );
    }
}
