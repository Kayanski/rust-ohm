use astroport::{asset::PairInfo, pair::PoolResponse};
use cosmwasm_std::Deps;

use crate::{
    state::{PAIR_INFO, POOL_INFO},
    ContractError,
};

/// This represents the value of each staking token compared to the base token
/// For instance, if this contracts hold 100 CW20 and has minted 80 sCW20, the exchange rate is 100/80 = 1.25
pub fn query_pair(deps: Deps) -> Result<PairInfo, ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;

    Ok(pair_info)
}

/// This represents the value of each staking token compared to the base token
/// For instance, if this contracts hold 100 CW20 and has minted 80 sCW20, the exchange rate is 100/80 = 1.25
pub fn query_pool(deps: Deps) -> Result<PoolResponse, ContractError> {
    let pool_info = POOL_INFO.load(deps.storage)?;

    Ok(pool_info)
}
