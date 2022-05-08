use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::models::Todo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddTodo {
        description: String,
    },
    EditTodo {
        id: u64,
        description: Option<String>,
        status: Option<u8>,
    },
    Delete {
        id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetList {
        addr: String,
        offset: Option<u64>,
        limit: Option<u64>,
    },
    GetTodo {
        id: u64,
        addr: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TodosResponse {
    pub todos: Vec<Todo>,
}
