use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cw_asset::{AssetBase, AssetInfo};

use crate::{
    helpers::deposit_one_coin,
    query::_available_unlock,
    state::{
        deposit::{deposits, DepositInfo, DepositLock},
        ACCEPTED_TOKENS, CONFIG,
    },
    ContractError,
};

pub fn execute_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
    token: AssetBase<String>,
    lock: DepositLock,
) -> Result<Response, ContractError> {
    let to_addr = deps.api.addr_validate(&to)?;
    let token = token.check(deps.api, None)?;

    let fee = ACCEPTED_TOKENS.load(deps.storage, &token.info).or(Err(
        ContractError::AssetNotAccepted {
            token: token.info.clone(),
        },
    ))?;

    // We apply the fee
    let token = fee.apply(token)?;

    // We save the deposit info
    let next_deposit_id = CONFIG
        .update(deps.storage, |mut c| {
            c.next_deposit_id += 1;
            Ok::<_, ContractError>(c)
        })?
        .next_deposit_id;

    deposits().save(
        deps.storage,
        next_deposit_id,
        &DepositInfo {
            id: next_deposit_id,
            lock,
            recipient: to_addr,
            asset: token.clone(),
        },
    )?;

    // We transfer the tokens
    let msg = match &token.info {
        AssetInfo::Cw20(_) => Some(token.transfer_from_msg(info.sender, env.contract.address)?),
        AssetInfo::Native(denom) => {
            let amount = deposit_one_coin(info, denom.to_string())?;
            if amount != token.amount {
                return Err(ContractError::NotEnoughDeposited {
                    expected: token.amount,
                    got: amount,
                });
            }
            None
        }
        _ => return Err(ContractError::AssetNotAccepted { token: token.info }),
    };

    Ok(Response::new().add_messages(msg))
}

pub fn execute_unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
    id: u64,
) -> Result<Response, ContractError> {
    let mut deposit_info = deposits().load(deps.storage, id)?;

    if info.sender != deposit_info.recipient && to != deposit_info.recipient {
        return Err(ContractError::Unauthorized {});
    }

    let available_deposit =
        _available_unlock(deps.as_ref(), &env, &deposit_info)?.unwrap_or(AssetBase {
            info: deposit_info.asset.info,
            amount: Uint128::zero(),
        });

    if available_deposit.amount.is_zero() {
        return Err(ContractError::NotEnoughDeposited {
            expected: Uint128::one(),
            got: Uint128::zero(),
        });
    }

    match deposit_info.lock {
        crate::state::deposit::DepositLock::Linear(ref mut lock) => {
            if lock.start < env.block.time {
                lock.start = env.block.time;
            }
            deposit_info.asset.amount -= available_deposit.amount;
        }
        crate::state::deposit::DepositLock::LinearProportion(ref mut lock) => {
            if lock.start < env.block.time {
                lock.start = env.block.time;
            }
            deposit_info.asset.amount -= available_deposit.amount;
        }
        crate::state::deposit::DepositLock::TimeUnlock(end) => {
            if end < env.block.time.seconds() {
                deposit_info.asset.amount -= available_deposit.amount;
            }
        }
    }

    let msg = available_deposit.transfer_msg(to)?;
    Ok(Response::new().add_message(msg))
}
