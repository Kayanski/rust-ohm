use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_asset::{AssetBase, AssetInfoBase};

use crate::state::Fee;

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
    },
    Unlock {
        to: String,
        asset: AssetBase<String>,
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
    #[returns(AssetBase<String>)]
    LockForToken {
        address: String,
        token: AssetInfoBase<String>,
    },
    #[returns(Vec<AssetBase<String>>)]
    LocksForAddress {
        address: String,
        start: Option<AssetInfoBase<String>>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: String,
}

#[cw_serde]
pub struct BondContractsResponse {
    pub bonds: Vec<BondContractsElem>,
}

#[cw_serde]
pub struct BondContractsElem {
    pub bond_token: String,
    pub bond_address: String,
}
