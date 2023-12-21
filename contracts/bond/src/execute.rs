use cosmos_sdk_proto::traits::Message;
use cosmwasm_std::{
    ensure, BankMsg, Coin, CosmosMsg, Decimal256, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, Uint256,
};
use injective_std::types::injective::tokenfactory::v1beta1::{MsgBurn, MsgMint};

use crate::{
    helpers::deposit_one_coin,
    query::{current_exchange_rate, staking_denom},
    state::{CONFIG, CURRENT_DEBT, LAST_DECAY, MARKETS, TERMS},
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
// pub fn create_market(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     quote_token: String,
//     capacity_in_quote: bool,
//     capacity: u128,
//     initial_price_per_ohm: Decimal256,
//     total_debt: u128,
//     sold: u128,
//     purchased: u128,
//     fixed_term: bool,
//     deposit_interval: u128,
//     control_variable: u128,
//     vesting_duration: u128,
//     conclusion: u128,
//     max_debt: u128,
//     max_to_target_debt_ratio: Decimal256,
// ) -> Result<Response, ContractError> {
//     let decimals = 6;
//     let markets = MARKETS.load(deps.storage)?;
//     let seconds_to_conclusion = conclusion - env.block.time;
//     let target_debt: Decimal256;
//     if (capacity_in_quote) {
//         target_debt = capacity;
//     } else {
//         target_debt = capacity / initial_price_per_ohm;
//     }
//     let max_payout_per_interval: Decimal256 =
//         target_debt * deposit_interval / seconds_to_conclusion;
//
//     let max_debt = target_debt * max_to_target_debt_ratio;
//     // let control_variable =
//
//     Ok(Response::new())
// }

pub fn decay_amount(deps: Deps, env: Env) -> Result<Uint128, StdError> {
    let terms = TERMS.load(deps.storage)?;
    let last_decay = LAST_DECAY.load(deps.storage)?;
    let time_since_last_decay = env.block.time.seconds() - last_decay.seconds();
    let current_debt = CURRENT_DEBT.load(deps.storage)?;

    let mut decay = current_debt * time_since_last_decay.into() / terms.vesting_term.into();
    if decay > current_debt {
        decay = current_debt;
    }
    Ok(decay)
}

pub fn decay_debt(deps: DepsMut, env: Env, info: MessageInfo) -> Result<(), ContractError> {
    CURRENT_DEBT.update(deps.storage, |d| Ok(d - decay_amount(deps.as_ref(), env)))?;
    LAST_DECAY.save(deps.storage, &env.block.time)?;

    Ok(())
}

pub fn get_bond_price_in_usd(deps: DepsMut, env: Env, info: MessageInfo) -> Decimal256 {
    get_asset_price(deps, env, info)
}

pub fn update_bond_price_in_terms(deps: DepsMut, env: Env, info: MessageInfo) -> Decimal256 {}

pub fn compute_deposit_value(deposited_amout: u128, is_lp: bool) -> u128 {
    if (!is_lp) {
        return deposited_amout;
    }
    // TODO : compute for LP
    return 0;
}
pub fn compute_payout(deposit_amount: u128, discount_ratio: Decimal256) -> u128 {
    deposit_amount / discount_ratio;
}

pub fn deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    max_price: Decimal256,
    depositor: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    decay_debt(deps, env, info);
    let current_debt = CURRENT_DEBT.load(deps.storage)?;
    let terms = TERMS.load(deps.storage)?;

    ensure!(
        current_debt <= terms.max_debt,
        StdError::generic_err("Max capacity reached")
    );

    let price_in_usd = bondPriceInUSD(); // Stored in bond info
    let native_price = _bondPrice();

    ensure!(
        max_price >= native_price,
        StdError::generic_err("Slippage limit: more than max price")
    ); // slippage protection

    let deposited_amount = deposit_one_coin(info, config.principle)?;

    if (deposited_amount == 0) {
        return ContractError::ReceiveOneCoin(config.principle);
    }
    if (current_debt >= terms.max_debt) {
        return ContractError::MaxDebtReachedError();
    }
    let is_lp = false;
    let deposit_value = compute_deposit_value(deposited_amount, is_lp);
    let payout = compute_payout(deposit_value, discount_ratio);

    Ok(Response::new())
}
