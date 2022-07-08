use crate::state::{Entry, Priority, Status};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    NewEntry {
        description: String,
        priority: Option<Priority>,
    },
    UpdateEntry {
        id: u64,
        description: Option<String>,
        status: Option<Status>,
        priority: Option<Priority>,
    },
    DeleteEntry {
        id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryEntry {
        id: u64,
    },
    QueryList {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EntryResponse {
    pub id: u64,
    pub description: String,
    pub status: Status,
    pub priority: Priority,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListResponse {
    pub entries: Vec<Entry>,
}
