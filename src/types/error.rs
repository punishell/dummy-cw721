use cosmwasm_std::StdError;
use thiserror::Error;

use crate::TokenId;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("token_id already claimed")]
    Claimed {},

    #[error("Cannot set approval that is already expired")]
    Expired {},

    #[error("Cannot remint a token ID that has already been used: {}", token_id)]
    RemintBurned { token_id: TokenId },

    #[error("The given token does not exist: {}", token_id)]
    NoSuchToken { token_id: TokenId },
}
