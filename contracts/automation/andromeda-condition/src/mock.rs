#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_automation::condition::{ExecuteMsg, InstantiateMsg, LogicGate};
use common::app::AndrAddress;
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_condition() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_condition_instantiate_msg(
    logic_gate: LogicGate,
    eval_ados: Vec<AndrAddress>,
    execute_ado: AndrAddress,
) -> InstantiateMsg {
    InstantiateMsg {
        logic_gate,
        eval_ados,
        execute_ado,
    }
}

pub fn mock_condition_get_results_msg() -> ExecuteMsg {
    ExecuteMsg::GetResults {}
}
