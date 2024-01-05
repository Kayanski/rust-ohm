use cosmos_sdk_proto::traits::Message;
use cosmwasm_std::{Coin, CosmosMsg, Env, MessageInfo, Uint128};
use injective_std::types::injective::tokenfactory::v1beta1::{MsgCreateDenom, MsgMint};

use crate::ContractError;

pub fn deposit_one_coin(info: MessageInfo, denom: String) -> Result<Uint128, ContractError> {
    // Verify the funds
    if info.funds.len() != 1 {
        return Err(ContractError::ReceiveOneCoin(denom));
    }
    let deposited_coin = &info.funds[0];
    // Verify the funds
    if deposited_coin.denom != denom {
        return Err(ContractError::ReceiveOneCoin(denom));
    }
    Ok(deposited_coin.amount)
}

pub fn create_denom_msg(env: &Env, subdenom: String) -> CosmosMsg {
    CosmosMsg::Stargate {
        type_url: MsgCreateDenom::TYPE_URL.to_string(),
        value: MsgCreateDenom {
            sender: env.contract.address.to_string(),
            subdenom,
        }
        .encode_to_vec()
        .into(),
    }
}

pub fn mint_msgs(
    env: &Env,
    denom: String,
    receiver: String,
    mint_amount: Uint128,
) -> [CosmosMsg; 2] {
    [
        CosmosMsg::Stargate {
            type_url: MsgMint::TYPE_URL.to_string(),
            value: MsgMint {
                sender: env.contract.address.to_string(),
                amount: Some(injective_std::types::cosmos::base::v1beta1::Coin {
                    denom: denom.clone(),
                    amount: mint_amount.to_string(),
                }),
            }
            .encode_to_vec()
            .into(),
        },
        CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
            to_address: receiver,
            amount: vec![Coin {
                denom,
                amount: mint_amount,
            }],
        }),
    ]
}
