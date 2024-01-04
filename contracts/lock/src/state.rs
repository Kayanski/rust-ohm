// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal256, Uint128, Uint256};
use cw_asset::Asset;
use cw_storage_plus::{Item, Map};

use crate::{msg::AcceptedTokenUnchecked, ContractError};

pub const CONFIG: Item<Config> = Item::new("config");
pub const ACCEPTED_TOKENS: Map<&cw_asset::AssetInfo, Fee> = Map::new("accepted_tokens");
pub const DEPOSITED_TOKENS: Map<(&Addr, &cw_asset::AssetInfo), Uint128> =
    Map::new("deposited_tokens");

#[cw_serde]
pub struct Config {
    pub admin: Addr,
}

#[cw_serde]
pub struct AcceptedToken {
    pub asset: cw_asset::AssetInfo,
    pub deposit_fee: Fee,
}

impl AcceptedTokenUnchecked {
    pub fn check(&self, api: &dyn Api) -> Result<AcceptedToken, ContractError> {
        Ok(AcceptedToken {
            asset: self.asset.check(api, None)?,
            deposit_fee: self.deposit_fee.clone(),
        })
    }
}

#[cw_serde]
pub enum Fee {
    Fixed(Uint128),
    Variable(Decimal256),
}

impl Fee {
    pub fn apply(&self, mut asset: Asset) -> Result<Asset, ContractError> {
        match self {
            Fee::Fixed(f) => {
                if f < &asset.amount {
                    return Err(ContractError::NotEnoughDeposited {
                        expected: *f,
                        got: asset.amount,
                    });
                }
                asset.amount -= f;
                Ok(asset)
            }
            Fee::Variable(v) => {
                asset.amount = (Uint256::from(asset.amount) * *v).try_into()?;
                Ok(asset)
            }
        }
    }
}

#[cw_serde]
pub struct TokenLock {
    pub asset: cw_asset::Asset,
    pub deposit_fee: Fee,
}
