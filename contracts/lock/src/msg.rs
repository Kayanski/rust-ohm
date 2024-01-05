use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_asset::{AssetBase, AssetInfoBase};

use crate::state::{
    deposit::{DepositInfo, DepositLock},
    Fee,
};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    pub accepted_tokens: Vec<AcceptedTokenUnchecked>,
}

#[cw_serde]
pub struct AcceptedTokenUnchecked {
    pub asset: cw_asset::AssetInfoBase<String>,
    pub deposit_fee: Fee,
}

/// Message type for `execute` entry_point
#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    UpdateAcceptedToken {
        to_add: Vec<AcceptedTokenUnchecked>,
        to_remove: Vec<cw_asset::AssetInfoBase<String>>,
    },
    #[cfg_attr(feature = "interface", payable)]
    Lock {
        to: String,
        asset: AssetBase<String>,
        lock: DepositLock,
    },
    Unlock {
        to: String,
        id: u64,
    },
    UpdateConfig {
        admin: Option<String>,
    },
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(Vec<AcceptedTokenUnchecked>)]
    AcceptedTokens {
        start: Option<AssetInfoBase<String>>,
        limit: Option<u32>,
    },
    #[returns(DepositInfoResponse)]
    Lock { id: u64 },
    #[returns(AssetBase<String>)]
    AvailableUnlock { id: u64 },
    #[returns(Vec<DepositInfoResponse>)]
    LocksForAddress {
        address: String,
        start: Option<u64>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: String,
}

#[cw_serde]
pub struct DepositInfoResponse {
    pub id: u64,
    pub lock: DepositLock,
    pub recipient: String,
    pub asset: AssetBase<String>,
}

impl From<DepositInfo> for DepositInfoResponse {
    fn from(value: DepositInfo) -> Self {
        Self {
            id: value.id,
            lock: value.lock,
            recipient: value.recipient.to_string(),
            asset: value.asset.into(),
        }
    }
}
