#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::rebase::execute_rebase;

/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(cw20_base::contract::instantiate(deps, env, info, msg.base)?)
}

/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Rebase { profit } => execute_rebase(deps, info, profit),
        _ => Ok(cw20_base::contract::execute(deps, env, info, msg.into())?),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CirculatingSupply {} => todo!(),
        // Find matched incoming message variant and query them your custom logic
        // and then construct your query response with the type usually defined
        // `msg.rs` alongside with the query message itself.
        //
        // use `cosmwasm_std::to_binary` to serialize query response to json binary.
    }
}

/// Handling submessage reply.
/// For more info on submessage and reply, see https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#submessages
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    // With `Response` type, it is still possible to dispatch message to invoke external logic.
    // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages

    todo!()
}

#[cfg(test)]
pub mod test {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use crate::msg::InstantiateMsg;

    use super::instantiate;

    #[test]
    pub fn init_works() -> anyhow::Result<()> {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("admin", &[]);

        instantiate(
            deps.as_mut(),
            env,
            info,
            InstantiateMsg {
                base: cw20_base::msg::InstantiateMsg {
                    name: "OHM".to_string(),
                    symbol: "OHM".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: None,
                    marketing: None,
                },
            },
        );

        Ok(())
    }
}
