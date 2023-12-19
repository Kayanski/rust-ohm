use cosmwasm_std::{Deps, Uint128};

use crate::ContractError;

pub fn circulating_supply(deps: Deps) -> Result<Uint128, ContractError> {
    Ok(cw20_base::contract::query_token_info(deps)?.total_supply)
}
