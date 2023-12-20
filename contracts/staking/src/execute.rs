use cosmos_sdk_proto::traits::Message;
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, Decimal256, DepsMut, Env, MessageInfo, Response, Uint256,
};
use injective_std::types::injective::tokenfactory::v1beta1::{MsgBurn, MsgMint};

use crate::{
    helpers::deposit_one_coin,
    query::{current_exchange_rate, staking_denom},
    state::CONFIG,
    ContractError,
};

pub const INITIAL_FRAGMENTS_SUPPLY: u128 = 5_000_000;
pub const TOTAL_GONS: u128 = u128::MAX - (u128::MAX % INITIAL_FRAGMENTS_SUPPLY);
pub const MAX_SUPPLY: u128 = u64::MAX as u128;

// pub fn rebase(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
//     let config = CONFIG.load(deps.storage)?;
//     let epoch_state = EPOCH_STATE.load(deps.storage)?;

//     if epoch_state.end > env.block.time {
//         return Ok(Response::new());
//     }

//     let msg = SubMsg::reply_on_success(
//         CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
//             contract_addr: config.sohm.to_string(),
//             msg: to_json_binary(&sCW20ExecuteMsg::Rebase {
//                 profit: epoch_state.distribute,
//             })?,
//             funds: vec![],
//         }),
//         AFTER_SOHM_REBASE_REPLY,
//     );

//     Ok(Response::new().add_submessage(msg))
// }

// pub fn rebase_reply(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
//     let config = CONFIG.load(deps.storage)?;

//     //         if (address(distributor) != address(0)) {
//     //             distributor.distribute();
//     //             bounty = distributor.retrieveBounty(); // Will mint ohm for this contract if there exists a bounty
//     //         }

//     EPOCH_STATE.update(deps.storage, |mut epoch_state| {
//         epoch_state.end = epoch_state.end.plus_seconds(config.epoch_length);
//         epoch_state.number += 1;

//         Ok::<_, StdError>(epoch_state)
//     })?;

//     Ok(Response::new())
// }

pub fn stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let deposited_amount = deposit_one_coin(info, config.ohm)?;

    let exchange_rate = current_exchange_rate(deps.as_ref(), &env, Some(deposited_amount), None)?;

    let mint_amount =
        (Decimal256::from_ratio(deposited_amount, 1u128) / exchange_rate) * Uint256::one();

    // We mint some sOHM to use
    let mint_msg = CosmosMsg::Stargate {
        type_url: MsgMint::TYPE_URL.to_string(),
        value: MsgMint {
            sender: env.contract.address.to_string(),
            amount: Some(injective_std::types::cosmos::base::v1beta1::Coin {
                denom: staking_denom(&env),
                amount: mint_amount.to_string(),
            }),
        }
        .encode_to_vec()
        .into(),
    };

    // And send to the depositors
    let send_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: to,
        amount: vec![Coin {
            amount: mint_amount.try_into()?,
            denom: staking_denom(&env),
        }],
    });
    // // We rebase after if needed
    // let rebase_msg = CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
    //     contract_addr: env.contract.address.to_string(),
    //     msg: to_json_binary(&ExecuteMsg::Rebase {})?,
    //     funds: vec![],
    // });

    Ok(Response::new().add_message(mint_msg).add_message(send_msg))
}

pub fn unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let deposited_amount = deposit_one_coin(info, staking_denom(&env))?;

    let exchange_rate = current_exchange_rate(deps.as_ref(), &env, None, Some(deposited_amount))?;

    let redeem_amount =
        (Decimal256::from_ratio(deposited_amount, 1u128) * exchange_rate) * Uint256::one();

    // We burn the received sOHM to the depositor
    let burn_msg = CosmosMsg::Stargate {
        type_url: MsgMint::TYPE_URL.to_string(),
        value: MsgBurn {
            sender: env.contract.address.to_string(),
            amount: Some(injective_std::types::cosmos::base::v1beta1::Coin {
                denom: staking_denom(&env),
                amount: deposited_amount.to_string(),
            }),
        }
        .encode_to_vec()
        .into(),
    };

    // We send OHM back to the depositor
    let send_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: to,
        amount: vec![Coin {
            amount: redeem_amount.try_into()?,
            denom: config.ohm,
        }],
    });

    Ok(Response::new().add_message(burn_msg).add_message(send_msg))
}
