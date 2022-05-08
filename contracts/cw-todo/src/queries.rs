use cosmwasm_std::{Deps, Order, StdError, StdResult};
use cw_storage_plus::Bound;

use crate::{models::Todo, msg::TodosResponse, state::TODOS};

const DEFAULT_LIMIT: u64 = 10;

pub fn query_todo(deps: Deps, id: u64, addr: String) -> StdResult<Todo> {
    let v_addr = deps.api.addr_validate(&addr)?;
    match TODOS.load(deps.storage, (&v_addr, id)) {
        Ok(todo) => Ok(todo),
        Err(_) => Err(StdError::not_found("todo")),
    }
}

pub fn query_list(
    deps: Deps,
    addr: String,
    offset: Option<u64>,
    limit: Option<u64>,
) -> StdResult<TodosResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let min: u64 = offset.unwrap_or(0);
    let max: u64 = min + limit;

    let v_addr = deps.api.addr_validate(&addr)?;

    let todos: StdResult<Vec<Todo>> = TODOS
        .prefix(&v_addr)
        .range(
            deps.storage,
            Some(Bound::inclusive(min)),
            Some(Bound::inclusive(max)),
            Order::Ascending,
        )
        .take(limit as usize)
        .map(|i| i.map(|(_, t)| t))
        .collect();

    Ok(TodosResponse { todos: todos? })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    use crate::contract::query;
    use crate::models::{Status, Todo};
    use crate::msg::{QueryMsg, TodosResponse};
    use crate::state::TODOS;

    #[test]
    fn get_todo() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &coins(1000, "token"));

        let id = 0;
        let todo = Todo {
            id: id.clone(),
            description: String::from("Improve tests"),
            status: Status::OPEN,
        };

        let _res = TODOS.save(deps.as_mut().storage, (&info.sender, id.clone()), &todo);

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetTodo {
                addr: info.sender.to_string(),
                id: id.clone(),
            },
        )
        .unwrap();
        let value: Todo = from_binary(&res).unwrap();
        assert_eq!(value, todo);
    }

    #[test]
    fn get_list() {
        let mut deps = mock_dependencies();
        let info = mock_info("creator", &coins(1000, "token"));

        let open_todo = Todo {
            id: 0,
            description: String::from("OPEN"),
            status: Status::OPEN,
        };
        let completed_todo = Todo {
            id: 1,
            description: String::from("COMPLETED"),
            status: Status::COMPLETED,
        };

        let _f1 = TODOS.save(deps.as_mut().storage, (&info.sender, 0), &open_todo);
        let _f2 = TODOS.save(deps.as_mut().storage, (&info.sender, 1), &completed_todo);

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetList {
                addr: info.sender.to_string(),
                offset: None,
                limit: None,
            },
        )
        .unwrap();
        let todos: TodosResponse = from_binary(&res).unwrap();
        assert_eq!(todos.todos, vec![open_todo, completed_todo]);
    }
}
