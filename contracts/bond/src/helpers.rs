use cosmwasm_std::{Decimal256, DepsMut, Env, MessageInfo, Uint128};

use crate::{
    state::{ADJUSTMENT, TERMS},
    ContractError,
};

pub fn deposit_one_coin(info: MessageInfo, denom: String) -> Result<Uint128, ContractError> {
    // Verify the funds
    if info.funds.len() != 1 {
        return Err(ContractError::ReceiveOneCoin(denom));
    }
    let deposited_coin = &info.funds[0];
    // Verify the funds
    if deposited_coin.denom != denom {
        return Err(ContractError::ReceiveOneCoin(denom));
    }
    Ok(deposited_coin.amount)
}

pub fn adjust(deps: DepsMut, env: Env) -> Result<(), ContractError> {
    let mut adjustment = ADJUSTMENT.load(deps.storage)?;
    let mut terms = TERMS.load(deps.storage)?;
    let time_can_adjust = adjustment.last_time.plus_seconds(adjustment.buffer);
    if !adjustment.rate.is_zero() && env.block.time > time_can_adjust {
        if adjustment.add {
            terms.control_variable += adjustment.rate;
            if terms.control_variable >= adjustment.target {
                adjustment.rate = Decimal256::zero();
            }
        } else {
            terms.control_variable = terms
                .control_variable
                .checked_sub(adjustment.rate)
                .unwrap_or_default();
            if terms.control_variable <= adjustment.target {
                adjustment.rate = Decimal256::zero();
            }
        }
        ADJUSTMENT.save(deps.storage, &adjustment)?;
        TERMS.save(deps.storage, &terms)?;
    }

    Ok(())
}
