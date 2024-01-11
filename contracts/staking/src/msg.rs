use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal256, Uint128};

use crate::state::{EpochState, StakingPoints};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    pub epoch_length: u64,
    pub epoch_apr: Decimal256,
    pub first_epoch_time: u64,
    pub initial_balances: Vec<(String, Uint128)>,
    pub warmup_length: u64,
}

/// Message type for `execute` entry_point
#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    InstantiateContracts {
        staking_token_code_id: u64,
        cw1_code_id: u64,
        staking_symbol: String,
        staking_name: String,
    },
    #[cfg_attr(feature = "interface", payable)]
    Stake {
        to: String,
    },
    Claim {
        to: String,
    },
    #[cfg_attr(feature = "interface", payable)]
    Unstake {
        to: String,
        amount: Uint128,
    },
    Rebase {},
    Mint {
        to: String,
        amount: Uint128,
    },
    UpdateConfig {
        admin: Option<String>,
        epoch_length: Option<u64>,
        epoch_apr: Option<Decimal256>,
        add_bond: Option<Vec<BondContractsElem>>,
        remove_bond: Option<Vec<String>>,
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
    #[returns(Decimal256)]
    ExchangeRate {},
    #[returns(BondContractsResponse)]
    Bonds {},
    #[returns(EpochState)]
    EpochState {},
    #[returns(StakingPoints)]
    StakingPoints { address: String },
    #[returns(StakingPoints)]
    RawStakingPoints { address: String },
}

#[cw_serde]
pub struct ConfigResponse {
    pub epoch_length: u64,
    pub epoch_apr: Decimal256,
    pub next_epoch_apr: Option<Decimal256>,
    pub admin: String,
    pub ohm_denom: String,
    pub sohm_address: String,
    pub warmup_length: u64,
    pub warmup_address: String,
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
