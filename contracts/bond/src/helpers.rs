use cosmwasm_std::{MessageInfo, Uint128};

use crate::ContractError;

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
