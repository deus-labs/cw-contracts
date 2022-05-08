use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use todo_list::models::{Status, Todo};
use todo_list::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TodosResponse};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(Status), &out_dir);
    export_schema(&schema_for!(Todo), &out_dir);
    export_schema(&schema_for!(TodosResponse), &out_dir);
}
