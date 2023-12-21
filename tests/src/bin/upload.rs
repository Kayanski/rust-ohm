use std::str::FromStr;

use cosmwasm_std::{coins, Decimal256};
use cw_orch::{
    contract::interface_traits::{CwOrchInstantiate, CwOrchUpload},
    daemon::DaemonBuilder,
    environment::TxHandler,
    tokio::runtime::Runtime,
};
use staking::{interface::Staking, msg::InstantiateMsg};
use tests::{AMOUNT_TO_CREATE_DENOM, LOCAL_INJECTIVE};

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    pretty_env_logger::init();
    let rt = Runtime::new()?;
    let chain = DaemonBuilder::default()
        .chain(LOCAL_INJECTIVE)
        .handle(rt.handle())
        .build()?;
    let contract = Staking::new("staking", chain.clone());
    contract.upload()?;

    contract.instantiate(
        &InstantiateMsg {
            admin: None,
            epoch_apr: Decimal256::from_str("1.1")?,
            first_epoch_time: 100_000,
            epoch_length: 100,
            initial_balances: vec![(chain.sender().to_string(), 1_000_000u128.into())],
        },
        None,
        Some(&coins(AMOUNT_TO_CREATE_DENOM * 2, "inj")),
    )?;

    Ok(())
}
