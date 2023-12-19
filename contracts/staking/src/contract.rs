use cosmos_sdk_proto::traits::Message;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Decimal256, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Timestamp, Uint128,
};
use injective_std::types::injective::tokenfactory::v1beta1::MsgCreateDenom;

use crate::error::ContractError;
use crate::execute::{stake, unstake};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, EpochState, CONFIG, EPOCH_STATE, STAKING_TOKEN_DENOM};

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

    // We create the staked currency denomination
    let msg = CosmosMsg::Stargate {
        type_url: MsgCreateDenom::TYPE_URL.to_string(),
        value: MsgCreateDenom {
            sender: env.contract.address.to_string(),
            subdenom: STAKING_TOKEN_DENOM.to_string(),
        }
        .encode_to_vec()
        .into(),
    };

    Ok(Response::new().add_message(msg))
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
    use cosmos_sdk_proto::{traits::Message, Any};
    use cosmwasm_std::coins;
    use cw_orch::{injective_test_tube::InjectiveTestTube, prelude::*};
    use injective_std::types::injective::tokenfactory::v1beta1::{
        MsgCreateDenom, MsgCreateDenomResponse,
    };

    use staking::interface::Staking;
    use staking::msg::InstantiateMsg;

    pub const MAIN_TOKEN: &str = "OHM";
    pub const AMOUNT_TO_CREATE_DENOM: u128 = 10_000_000_000_000_000_000u128;

    #[test]
    pub fn init_works() -> anyhow::Result<()> {
        let chain = InjectiveTestTube::new(coins(AMOUNT_TO_CREATE_DENOM * 2, "inj"));

        // First we need to create the OHM denom
        chain.commit_any::<MsgCreateDenomResponse>(
            vec![Any {
                type_url: MsgCreateDenom::TYPE_URL.to_string(),
                value: MsgCreateDenom {
                    sender: chain.sender().to_string(),
                    subdenom: MAIN_TOKEN.to_string(),
                }
                .encode_to_vec(),
            }],
            None,
        )?;

        let contract = Staking::new("staking", chain.clone());
        contract.upload()?;

        contract.instantiate(
            &InstantiateMsg {
                ohm: format!("{}/{MAIN_TOKEN}", chain.sender()),
                epoch_length: 2000,
                first_epoch_number: 100_000_000,
                first_epoch_time: 100_000_000,
                admin: None,
            },
            None,
            None,
        )?;

        Ok(())
    }
}
