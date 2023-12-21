use cosmwasm_std::{Addr, StdError};
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

    #[error("You need to send exactly one coin with this function with denom {0}")]
    ReceiveOneCoin(String),

    #[error(transparent)]
    ConversionOverflowError(#[from] cosmwasm_std::ConversionOverflowError),
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
