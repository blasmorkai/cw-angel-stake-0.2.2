use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Binary, Coin, Decimal, Uint128, Addr};
use cw20::Expiration;
use cw20::{AllowanceResponse, BalanceResponse, TokenInfoResponse};
pub use cw_controllers::ClaimsResponse;
use cw_utils::Duration;
use crate::state::{Validator_Info, Validator_Deposits};

#[cw_serde]
pub struct InstantiateMsg {
   pub agent: String,	//Address
   pub manager: String, //Address
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Bond will bond all staking tokens sent with the message
    Bond {nft_id: Uint128, amount: Uint128},
    /// send the unbonded staking tokens to the message sender
    Unbond { nft_id: Uint128, amount: Uint128 },
    /// Claim is used to claim your native tokens that you previously "unbonded"
    /// after the chain-defined waiting period (eg. 3 weeks)
    Claim {nft_id: Uint128 },
    /// Implements CW20 "approval" extension. Allows spender to access an additional amount tokens
    /// from the owner's (env.sender) account. If expires is Some(), overwrites current allowance
    /// expiration with this one.
    AddValidator {address: String, bond_denom: String, unbonding_period: Duration},
    RemoveValidator {address: String},
    BondCheck {},
}


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Claims shows the number of tokens this address can access when they are done unbonding.
    // To implement Claims for nft_id
    #[returns(ClaimsResponse)]
    Claims { nft_id: String },
    #[returns(Validator_Info)]
    ValidatorInfo {address: String},
    #[returns(Validator_Deposits)]
    ValidatorDeposits {address: String},
    #[returns(Uint128)]
    TotalBonded {},
    #[returns(Uint128)]
    TotalClaimed {},
    #[returns(String)]
    Agent {},   
    #[returns(String)]
    Manager {},       
}



