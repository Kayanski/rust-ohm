use crate::deploy::upload::Shogun;
use cw20::msg::Cw20ExecuteMsgFns as _;
use cw_orch::environment::CwEnv;
use cw_orch::prelude::ContractInstance;
use staking_contract::msg::ExecuteMsgFns as _;

pub mod deploy;
pub mod test_constants {
    pub const USD: &str = "uusd";
    pub const EPOCH_LENGTH: u64 = 1_000;
    pub const EPOCH_APR: &str = "0.1";
    pub const INITIAL_SHOGUN_BALANCE: u128 = 50_000_000_000;

    pub mod bond_terms_1 {

        pub const BOND_TOKEN: &str = "inj";
        pub const CONTROL_VARIABLE: &str = "70000000";
        pub const MAX_DEBT: u128 = 100_000_000_000_000;
        pub const MAX_PAYOUT: &str = "100000000";
        pub const MINIMUM_PRICE: &str = "2";
        pub const VESTING_TERM: u64 = 3600;
    }
}

pub fn unstake<Chain: CwEnv>(
    shogun: &Shogun<Chain>,
    amount: u128,
    receiver: Option<String>,
) -> anyhow::Result<()> {
    shogun.staking_token.increase_allowance(
        amount.into(),
        shogun.staking.address()?.to_string(),
        None,
    )?;
    shogun.staking.unstake(
        amount.into(),
        receiver.unwrap_or_else(|| shogun.staking.get_chain().sender().to_string()),
        &[],
    )?;

    Ok(())
}
