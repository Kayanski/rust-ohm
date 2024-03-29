use anyhow::bail;
use cosmos_sdk_proto::traits::Message;
use cw_orch::environment::CwEnv;
use cw_orch::prelude::*;
use injective_std::types::cosmos::base::v1beta1::Coin;
use injective_std::types::injective::tokenfactory::v1beta1::MsgCreateDenom;
use injective_std::types::injective::tokenfactory::v1beta1::MsgCreateDenomResponse;
use injective_std::types::injective::tokenfactory::v1beta1::MsgMint;
use injective_std::types::injective::tokenfactory::v1beta1::MsgMintResponse;
use prost_types::Any;
use staking_token::interface::StakingToken;
pub fn create_denom<Chain: CwEnv + Stargate>(
    chain: Chain,
    denom: String,
) -> anyhow::Result<<Chain as TxHandler>::Response>
where
    <Chain as TxHandler>::Error: Sync + Send + std::error::Error + 'static,
{
    // First we need to create the OHM denom
    Ok(chain.commit_any::<MsgCreateDenomResponse>(
        vec![Any {
            type_url: MsgCreateDenom::TYPE_URL.to_string(),
            value: MsgCreateDenom {
                sender: chain.sender().to_string(),
                subdenom: denom,
            }
            .encode_to_vec(),
        }],
        None,
    )?)
}

pub fn mint_denom<Chain: CwEnv + Stargate>(
    chain: Chain,
    denom: String,
    amount: u128,
) -> anyhow::Result<<Chain as TxHandler>::Response>
where
    <Chain as TxHandler>::Error: Sync + Send + std::error::Error + 'static,
{
    let sender = chain.sender().to_string();
    // First we need to create the OHM denom
    Ok(chain.commit_any::<MsgMintResponse>(
        vec![Any {
            type_url: MsgMint::TYPE_URL.to_string(),
            value: MsgMint {
                sender: sender.clone(),
                amount: Some(Coin {
                    denom: tokenfactory_denom(chain.clone(), denom),
                    amount: amount.to_string(),
                }),
            }
            .encode_to_vec(),
        }],
        None,
    )?)
}

pub fn tokenfactory_denom<Chain: CwEnv + Stargate>(chain: Chain, denom: String) -> String
where
    <Chain as TxHandler>::Error: Sync + Send + std::error::Error + 'static,
{
    let sender = chain.sender().to_string();

    format!("factory/{}/{}", sender, denom)
}

pub fn assert_balance<Chain: CwEnv>(
    chain: Chain,
    denom: String,
    amount: u128,
    address: String,
) -> anyhow::Result<()>
where
    <Chain as TxHandler>::Error: Sync + Send + std::error::Error + 'static,
{
    let current_balance = chain.balance(address, Some(denom))?;
    let balance = current_balance
        .first()
        .ok_or(anyhow::format_err!("No balance of this coin registered"))?;

    if balance.amount.u128() != amount {
        bail!("Wrong balance, Expected {}, got {}", amount, balance.amount);
    }

    Ok(())
}

use cw20_base::msg::QueryMsgFns as _;
pub fn assert_cw20_balance<Chain: CwEnv>(
    chain: Chain,
    token_address: String,
    amount: u128,
    address: String,
) -> anyhow::Result<()>
where
    <Chain as TxHandler>::Error: Sync + Send + std::error::Error + 'static,
{
    let balance = get_cw20_balance(chain, token_address, address)?;

    if balance != amount {
        bail!("Wrong balance, Expected {}, got {}", amount, balance);
    }

    Ok(())
}

pub fn get_cw20_balance<Chain: CwEnv>(
    chain: Chain,
    token_address: String,
    address: String,
) -> anyhow::Result<u128>
where
    <Chain as TxHandler>::Error: Sync + Send + std::error::Error + 'static,
{
    let token = StakingToken::new("token_address", chain.clone());
    token.set_address(&Addr::unchecked(token_address));

    let current_balance = token.balance(address)?;
    let balance = current_balance.balance.u128();
    Ok(balance)
}
