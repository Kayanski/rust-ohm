use cosmwasm_std::{Addr, StdError, Uint128};
use cw_asset::AssetInfo;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Unauthorized, only {address} is authorized")]
    UnauthorizedWithAddress { address: Addr },

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },

    #[error(transparent)]
    Cw20Error(#[from] cw20_base::ContractError),

    #[error(transparent)]
    AssetError(#[from] cw_asset::AssetError),

    #[error("You need to send exactly one coin with this function with denom {0}")]
    ReceiveOneCoin(String),

    #[error(transparent)]
    ConversionOverflowError(#[from] cosmwasm_std::ConversionOverflowError),

    #[error("Invalid reply id on rply endpoint")]
    InvalidReplyId {},

    #[error("No deposit found for address {address} and token {token}")]
    NoDepositInfo { address: Addr, token: AssetInfo },

    #[error("Asset Not authorized {token}")]
    AssetNotAccepted { token: AssetInfo },

    #[error("Not enough assets deposited expected: {expected}, got: {got}")]
    NotEnoughDeposited { expected: Uint128, got: Uint128 },
}
