use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw_asset::{AssetBase, AssetInfo};

use crate::{
    helpers::deposit_one_coin,
    state::{ACCEPTED_TOKENS, DEPOSITED_TOKENS},
    ContractError,
};

pub fn lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
    token: AssetBase<String>,
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
    DEPOSITED_TOKENS.update(deps.storage, (&to_addr, &token.info), |deposit| {
        let deposit = deposit.unwrap_or_default() + token.amount;
        Ok::<_, ContractError>(deposit)
    })?;

    // We transfer the tokens
    let msg = match &token.info {
        AssetInfo::Cw20(_) => Some(token.transfer_from_msg(info.sender, env.contract.address)?),
        AssetInfo::Native(denom) => {
            let amount = deposit_one_coin(info, denom.to_string())?;
            if amount != token.amount {
                return Err(ContractError::AssetNotAccepted { token: token.info });
            }
            None
        }
        _ => return Err(ContractError::AssetNotAccepted { token: token.info }),
    };

    Ok(Response::new().add_messages(msg))
}

pub fn unlock(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    to: String,
    token: AssetBase<String>,
) -> Result<Response, ContractError> {
    let token = token.check(deps.api, None)?;

    DEPOSITED_TOKENS.update(deps.storage, (&info.sender, &token.info), |deposit| {
        let deposit = deposit.unwrap_or_default();

        if deposit < token.amount {
            Err(ContractError::NotEnoughDeposited {
                expected: token.amount,
                got: deposit,
            })
        } else {
            Ok(token.amount - deposit)
        }
    })?;

    let msg = token.transfer_msg(to)?;
    Ok(Response::new().add_message(msg))
}
