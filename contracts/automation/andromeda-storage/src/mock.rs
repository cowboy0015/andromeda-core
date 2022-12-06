#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_automation::storage::{ExecuteMsg, InstantiateMsg};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_storage() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_storage_instantiate_msg(
    task_balancer: Addr,
    process: Addr,
    max_processes: u64,
) -> InstantiateMsg {
    InstantiateMsg {
        task_balancer,
        process,
        max_processes,
    }
}

pub fn mock_storage_store_msg(process: String) -> ExecuteMsg {
    ExecuteMsg::Store { process }
}

pub fn mock_storage_remove_msg(process: String) -> ExecuteMsg {
    ExecuteMsg::Remove { process }
}
