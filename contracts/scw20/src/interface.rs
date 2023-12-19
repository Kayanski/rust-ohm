use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::Empty;
use cw_orch::{interface, prelude::*};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
pub struct SCW20;

impl<Chain: CwEnv> Uploadable for SCW20<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(&self) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("scw20")
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper(&self) -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        ))
    }
}
