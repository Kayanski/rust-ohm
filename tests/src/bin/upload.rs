use std::str::FromStr;

use cosmwasm_std::Decimal256;
use cw_orch::{
    daemon::{networks::INJECTIVE_888, DaemonBuilder},
    deploy::Deploy,
    environment::TxHandler,
    tokio::runtime::Runtime,
};
use tests::deploy::upload::{Shogun, ShogunDeployment};

pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    pretty_env_logger::init();
    let rt = Runtime::new()?;

    let mut net = INJECTIVE_888;
    net.grpc_urls = &["http://injective-testnet-grpc.polkachu.com:14390"];
    let chain = DaemonBuilder::default()
        .chain(net)
        .handle(rt.handle())
        .build()?;

    let block_info = chain.block_info()?;

    // let shogun = Shogun::new(chain.clone());
    let shogun = Shogun::store_on(chain.clone())?;
    shogun.instantiate(ShogunDeployment {
        base_asset: "uusd".to_string(),
        epoch_length: 100,
        first_epoch_time: block_info.time.seconds() + 1,
        epoch_apr: Decimal256::from_str("0.1")?,
        initial_balances: vec![(chain.sender().to_string(), 1_000_000u128)],
        amount_to_create_denom: 1_000_000_000_000_000_000, // 1 INJ on testnet, 10 INJ on Mainnet
        fee_token: "inj".to_string(),
        staking_symbol: "sSHGN".to_string(),
        staking_name: "sSHOGUN".to_string(),
    })?;

    Ok(())
}
