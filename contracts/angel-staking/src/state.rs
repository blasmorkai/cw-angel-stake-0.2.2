use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128, Uint64};
use cw_controllers::Claims;
use cw_storage_plus::{Item, Map};
use cw_utils::Duration;


#[cw_serde]
pub struct Validator_Deposits {
    pub address:  String,
    /// bonded is how many native tokens exist bonded to the validator
    pub bonded: Uint128,
    /// claimed is how many native tokens exist claimed from the validator
    pub claimed: Uint128,
}

#[cw_serde]
pub struct Validator_Info{
    pub address:  String,
    /// Denomination we can stake
    pub bond_denom: String,
    /// unbonding period of the native staking module
    pub unbonding_period: Duration,
    pub total_bonded: Uint128,
	    // Needed or not needed, let's see
	    pub min_withdraw: Uint64,
}

// validator_addr, validator_deposits
pub const VALIDATOR_DEPOSITS: Map<&str, Validator_Deposits> = Map::new("validator_deposits");

// validator_addr, validator_info
pub const VALIDATOR_INFO: Map<&str, Validator_Info> = Map::new("validator_deposits");

// validator_addr, total_bonded_to_validator
// pub const VALIDATOR_BOND_AMOUNT: Map<&str, Uint128> = Map::new("validator_bond_amount");

//This is the unbonding period of the native staking module
pub const DENOM_UNBONDING_PERIOD : Map<String, Duration> = Map::new("unbonding_period");

pub const TOTAL_BONDED: Item<Uint128> = Item::new("total_deposits");

pub const TOTAL_CLAIMED: Item<Uint128> = Item::new("total_claims");

// Agent_addr
pub const AGENT: Item<String> = Item::new("relayer");

// angel_manager_addr
pub const MANAGER: Item<String> = Item::new("manager");

// A Claim allows a given address to claim an amount of tokens after a release date. 
// When a claim is created: an address, amount and expiration are given.
// POSSIBLE TO HAVE TO IMPLEMENT Claim(Map<&NFT_ID, Vec<Claim>>)   <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
// Claims(Map<&Addr, Vec<Claim>>)      struct Claim {amount: Uint128,release_at: Expiration,}

pub const CLAIMS: Claims = Claims::new("claims");

