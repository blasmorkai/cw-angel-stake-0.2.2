use cosmwasm_std::{StdError, Uint128, Uint64};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Validator '{validator}' not in current validator set")]
    NotInValidatorSet { validator: String },

    #[error("Different denominations in bonds: '{denom1}' vs. '{denom2}'")]
    DifferentBondDenom { denom1: String, denom2: String },

    #[error("Stored bonded {stored}, but query bonded {queried}")]
    BondedMismatch { stored: Uint128, queried: Uint128 },

    #[error("No {denom} tokens sent")]
    EmptyBalance { denom: String },

    #[error("Must unbond at least {min_bonded} {denom}")]
    UnbondTooSmall { min_bonded: Uint128, denom: String },

    #[error("Insufficient balance in contract to process claim")]
    BalanceTooSmall {},

    #[error("No claims that can be released currently")]
    NothingToClaim {},

    #[error("Cannot set to own account")]
    CannotSetOwnAccount {},

    #[error("Invalid expiration")]
    InvalidExpiration {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Allowance is expired")]
    Expired {},

    #[error("No funds sent")]
    NoFunds {},

    #[error("Multiple denoms sent")]
    MultipleDenoms {},

    #[error("Invalid Coin")]
    InvalidCoin {},

    #[error("Validator '{validator}' has already been registered to this contract")]
    ValidatorAlreadyRegistered { validator: String },

    #[error("Validator '{denom}' has already been registered to this contract")]
    DenominationCanNotBeBonded { denom: String },

    #[error("Bonded difference: Chain bonded - {total_bonded} , contract bonded - {state_total_bonded}")]
    BondedDiffer { total_bonded: Uint128, state_total_bonded: Uint128 },

    #[error("Unable to unstake {amount} from {number_validators} validators")]
    UnableUnstakeAmount { amount: Uint128, number_validators: Uint64 },
 
    #[error("Validator {address} not registered")]
    NotRegisteredValidator { address: String },

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
}