use cw_orch::{
    environment::{CwEnv, TxHandler},
    prelude::Stargate,
};
use injective_std::types::injective::exchange::v1beta1::{
    MsgInstantSpotMarketLaunch, MsgInstantSpotMarketLaunchResponse,
};
use prost::Message;
use prost_types::Any;

pub fn create_exchange<Chain>(
    chain: &Chain,
    base_denom: String,
    quote_denom: String,
) -> anyhow::Result<()>
where
    <Chain as TxHandler>::Error: Sync + Send + std::error::Error + 'static,
    Chain: CwEnv + Stargate,
{
    chain.commit_any::<MsgInstantSpotMarketLaunchResponse>(
        vec![Any {
            type_url: MsgInstantSpotMarketLaunch::TYPE_URL.to_string(),
            value: MsgInstantSpotMarketLaunch {
                sender: chain.sender().to_string(),
                ticker: format!("{}-{}", base_denom, quote_denom),
                base_denom,
                quote_denom,
                min_price_tick_size: "0".to_string(),
                min_quantity_tick_size: "0".to_string(),
            }
            .encode_to_vec(),
        }],
        None,
    )?;

    Ok(())
}
