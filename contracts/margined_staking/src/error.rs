use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("{0} is not native token")]
    NotNativeToken(String),

    #[error("Zero Division Error")]
    DivideByZero {},

    #[error("Expired")]
    Expired {},

    #[error("Invalid funds")]
    InvalidFunds {},

    #[error("Contract is already open")]
    IsOpen {},

    #[error("Invalid duration cannot be greater than {0}")]
    InvalidDuration(u64),

    #[error("Invalid ownership, new owner cannot be the same as existing")]
    InvalidOwnership {},

    #[error("Owner not set")]
    NoOwner {},

    #[error("Contract is not paused")]
    NotPaused {},

    #[error("Proposal not found")]
    ProposalNotFound {},

    #[error("Cannot perform action as contract is paused")]
    Paused {},

    #[error("Unauthorized")]
    Unauthorized {},
}
