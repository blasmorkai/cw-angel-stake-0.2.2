use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ Uint128, Uint64};
use cw_controllers::Claims;
use cw_storage_plus::{Item, Map, MultiIndex, Index, IndexList, IndexedMap};
use cw_utils::Duration;


//Unbonding period of the native staking module
// pub const UNBONDING_PERIOD : Map<String, Duration> = Map::new("unbonding_period");

//Denom of the native staking module
// pub const DENOM : Map<String, Duration> = Map::new("unbonding_period");

// Currently bonded and claimed
pub const BONDED: Item<Uint128> = Item::new("bonded");
pub const CLAIMED: Item<Uint128> = Item::new("claimed");

// All bonded and claimed 
pub const TOTAL_BONDED: Item<Uint128> = Item::new("total_bonded");
pub const TOTAL_CLAIMED: Item<Uint128> = Item::new("total_claimed");

pub const NUMBER_VALIDATORS: Item<Uint64> = Item::new("number_validators");

// Agent Addr
pub const AGENT: Item<String> = Item::new("relayer");

// Angel Manager Addr
pub const MANAGER: Item<String> = Item::new("manager");


// Claims(Map<&Addr, Vec<Claim>>)      struct Claim {amount: Uint128,release_at: Expiration,}
pub const CLAIMS: Claims = Claims::new("claims");

#[cw_serde]
pub struct ValidatorInfo{
    //pub address:  String,
    /// Denomination we can stake
    pub bond_denom: String,
    /// unbonding period of the native staking module
    pub unbonding_period: Duration,
    pub bonded: u128,
    pub claimed: u128,
}

pub struct ValidatorIndexes<'a> {
    pub bonded: MultiIndex<'a, u128, ValidatorInfo, &'a str>,
    pub claimed: MultiIndex<'a, u128, ValidatorInfo, &'a str>,
}

// This impl seems to be general
impl<'a> IndexList<ValidatorInfo> for ValidatorIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<ValidatorInfo>> + '_> {
        let v: Vec<&dyn Index<ValidatorInfo>> = vec![&self.bonded, &self.claimed];
        Box::new(v.into_iter())
    }
}

pub struct State <'a>
{
    // pk: validator address
    pub validator: IndexedMap<'a, &'a str, ValidatorInfo, ValidatorIndexes<'a>>,
}

impl<'a> State<'a>
{
    pub fn new() -> Self {
        Self {
            // pk: primary key -- d: data
            validator: IndexedMap::new(
                "validator_info",
            ValidatorIndexes { 
                bonded: MultiIndex::new(|_pk,d| d.bonded.clone(),"validatorinfo","validatorinfo__bonded"),
                claimed: MultiIndex::new(|_pk,d| d.claimed.clone(),"validatorinfo","validatorinfo__claimed"),
                },
            )
        }
    }
}
