// use `cw_storage_plus` to create ORM-like interface to storage
// see: https://crates.io/crates/cw-storage-plus

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal256, Uint128, Uint256};
use cw_asset::{Asset, AssetInfo};
use cw_storage_plus::{Item, Map};

use crate::{msg::AcceptedTokenUnchecked, ContractError};

pub const CONFIG: Item<Config> = Item::new("config");
pub const ACCEPTED_TOKENS: Map<&AssetInfo, Fee> = Map::new("accepted_tokens");

pub mod deposit {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::Addr;
    use cw_asset::Asset;
    use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};

    use self::locks::{LinearLock, LinearPercentageLock};

    #[cw_serde]
    #[non_exhaustive]
    pub enum DepositLock {
        Linear(LinearLock),
        LinearProportion(LinearPercentageLock),
        TimeUnlock(u64),
    }

    #[cw_serde]
    pub struct DepositInfo {
        pub id: u64,
        pub lock: DepositLock,
        pub recipient: Addr,
        pub asset: Asset,
    }

    pub struct DepositIndexes<'a> {
        pub recipient: MultiIndex<'a, Addr, DepositInfo, u64>,
    }

    impl<'a> IndexList<DepositInfo> for DepositIndexes<'a> {
        fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<DepositInfo>> + '_> {
            let v: Vec<&dyn Index<DepositInfo>> = vec![&self.recipient];
            Box::new(v.into_iter())
        }
    }

    pub fn deposits<'a>() -> IndexedMap<'a, u64, DepositInfo, DepositIndexes<'a>> {
        let indexes = DepositIndexes {
            recipient: MultiIndex::new(
                |_pk, d: &DepositInfo| d.recipient.clone(),
                "deposits",
                "deposits__recipient",
            ),
        };
        IndexedMap::new("deposits", indexes)
    }

    pub mod locks {
        use cosmwasm_schema::cw_serde;
        use cosmwasm_std::{Decimal256, Timestamp};
        #[cw_serde]
        pub struct LinearLock {
            pub start: Timestamp,
            pub per_second_vesting: Decimal256,
        }
        #[cw_serde]
        pub struct LinearPercentageLock {
            pub start: Timestamp,
            pub per_second_percent_vesting: Decimal256,
        }
    }
}

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub next_deposit_id: u64,
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
