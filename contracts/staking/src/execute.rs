use cosmos_sdk_proto::traits::Message;
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, Decimal256, DepsMut, Env, MessageInfo, Response, Uint128, Uint256,
};
use injective_std::types::injective::tokenfactory::v1beta1::{MsgBurn, MsgMint};

use crate::{
    helpers::{deposit_one_coin, mint_msgs},
    query::{base_denom, current_exchange_rate, staking_denom, token_balance},
    state::{CONFIG, EPOCH_STATE, MINTER_INFO},
    ContractError,
};

pub const INITIAL_FRAGMENTS_SUPPLY: u128 = 5_000_000;
pub const TOTAL_GONS: u128 = u128::MAX - (u128::MAX % INITIAL_FRAGMENTS_SUPPLY);
pub const MAX_SUPPLY: u128 = u64::MAX as u128;

pub fn rebase(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut epoch_state = EPOCH_STATE.load(deps.storage)?;

    if epoch_state.epoch_end > env.block.time {
        return Ok(Response::new());
    }

    // We mint some new OHM
    let current_balance = token_balance(deps.as_ref(), &env)?;
    let rebase_amount = Uint256::from(current_balance) * config.epoch_apr;

    // Mint some new ohm to this contract : this is where the APR coms from !
    let msg = CosmosMsg::Stargate {
        type_url: MsgMint::TYPE_URL.to_string(),
        value: MsgMint {
            sender: env.contract.address.to_string(),
            amount: Some(injective_std::types::cosmos::base::v1beta1::Coin {
                denom: base_denom(&env),
                amount: rebase_amount.to_string(),
            }),
        }
        .encode_to_vec()
        .into(),
    };

    epoch_state.epoch_end = epoch_state.epoch_end.plus_seconds(config.epoch_length);
    epoch_state.epoch_number += 1;

    EPOCH_STATE.save(deps.storage, &epoch_state)?;

    Ok(Response::new().add_message(msg))
}

pub fn stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
) -> Result<Response, ContractError> {
    let deposited_amount = deposit_one_coin(info, base_denom(&env))?;

    let exchange_rate = current_exchange_rate(deps.as_ref(), &env, Some(deposited_amount), None)?;

    let mint_amount =
        (Decimal256::from_ratio(deposited_amount, 1u128) / exchange_rate) * Uint256::one();

    // We mint some sOHM to the to address
    let mint_msgs = mint_msgs(&env, staking_denom(&env), to, mint_amount.try_into()?);

    Ok(Response::new().add_messages(mint_msgs))
}

pub fn unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
) -> Result<Response, ContractError> {
    let deposited_amount = deposit_one_coin(info, staking_denom(&env))?;

    let exchange_rate = current_exchange_rate(deps.as_ref(), &env, None, Some(deposited_amount))?;

    let redeem_amount =
        (Decimal256::from_ratio(deposited_amount, 1u128) * exchange_rate) * Uint256::one();

    // We burn the received sOHM from this contract
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
            denom: base_denom(&env),
        }],
    });

    Ok(Response::new().add_message(burn_msg).add_message(send_msg))
}

pub fn mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let minter_info = MINTER_INFO.load(deps.storage, info.sender)?;
    if !minter_info.can_mint {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::new().add_messages(mint_msgs(&env, base_denom(&env), to, amount)))
}
