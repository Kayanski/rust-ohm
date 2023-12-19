#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Decimal256, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Timestamp, Uint128,
};

use crate::error::ContractError;
use crate::execute::{stake, unstake};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, EpochState, CONFIG, EPOCH_STATE};

/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        ohm: msg.ohm,
        epoch_length: msg.epoch_length,
        admin: msg
            .admin
            .map(|addr| deps.api.addr_validate(&addr))
            .transpose()?
            .unwrap_or(info.sender),
    };
    let state = EpochState {
        number: msg.first_epoch_number,
        end: Timestamp::from_seconds(msg.first_epoch_time),
        distribute: Uint128::zero(),
        current_exchange_rate: Decimal256::one(),
    };

    CONFIG.save(deps.storage, &config)?;
    EPOCH_STATE.save(deps.storage, &state)?;

    Ok(Response::new())
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
        ExecuteMsg::Stake { to } => stake(deps, env, info, to),
        ExecuteMsg::Unstake { to } => unstake(deps, env, info, to),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Find matched incoming message variant and query them your custom logic
        // and then construct your query response with the type usually defined
        // `msg.rs` alongside with the query message itself.
        //
        // use `cosmwasm_std::to_binary` to serialize query response to json binary.
    }
}

// pub const AFTER_SOHM_REBASE_REPLY: u64 = 1;
// /// Handling submessage reply.
// /// For more info on submessage and reply, see https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#submessages
// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
//     match msg.id {
//         AFTER_SOHM_REBASE_REPLY => rebase_reply(deps, env),
//     }
// }

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
                ohm: todo!(),
                epoch_length: todo!(),
                first_epoch_number: todo!(),
                first_epoch_time: todo!(),
                admin: todo!(),
            },
        )?;

        Ok(())
    }
}
