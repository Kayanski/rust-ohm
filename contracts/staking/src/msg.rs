use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal256;

use crate::state::Config;

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub ohm: String, // Native token denom
    pub admin: Option<String>,
}

/// Message type for `execute` entry_point
#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    #[cfg_attr(feature = "interface", payable)]
    Stake { to: String },
    #[cfg_attr(feature = "interface", payable)]
    Unstake { to: String },
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    #[returns(Decimal256)]
    ExchangeRate {},
}
