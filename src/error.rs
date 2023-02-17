use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Never")]
    Never {},

    // funds errors
    #[error("Need {req_amount_denoms} assets deposited")]
    MismatchAmountDenoms { req_amount_denoms: u32 },

    #[error("Funds amounts must be equal")]
    FundsAmountNotEqual {},

    #[error("The two denoms in funds must be different")]
    FundsDenomAreSame {},

    #[error("Invalid denom: {denom}")]
    InvalidDenom { denom: String },

    #[error("Requested amount in start unbond higher than amount bonded assets")]
    StartUnbondAmountTooHigh {},

    #[error("Requested amount in unbond higher than amount available unbonded assets")]
    UnbondAmountTooHigh {},

    // state access error
    #[error("ID is already allocated")]
    IdAlreadyAllocated {},

    #[error("ID is not allocated")]
    IdNotAllocated {},

    #[error("No previous bonding data")]
    NoPreviousBondData {},

    // logic flow
    #[error("Cannot start unbonding if existing unconfirmed unbonding")]
    NoStartUnbondingIfExistingUnconfirmed {},
}
