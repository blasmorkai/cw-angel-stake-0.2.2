
// #[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env,
    MessageInfo, QuerierWrapper, Response, StakingMsg, StdResult, Uint128, Uint64,
    Order, Coin, DistributionMsg, CosmosMsg,
};

use cw2::set_contract_version;
use cw_utils::{one_coin, PaymentError, Duration, Expiration};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg,  QueryMsg};
use crate::state::{BONDED, CLAIMED, TOTAL_BONDED, TOTAL_CLAIMED, AGENT, MANAGER, CLAIMS, State, NUMBER_VALIDATORS, ValidatorInfo, TREASURY };

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-staking-angel";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    deps.api.addr_validate(&msg.manager)?;
    deps.api.addr_validate(&msg.agent)?;
    deps.api.addr_validate(&msg.treasury)?;
    
    AGENT.save(deps.storage, &msg.agent)?;
    MANAGER.save(deps.storage, &msg.manager)?;
    TREASURY.save(deps.storage, &msg.treasury)?;
    BONDED.save(deps.storage, &Uint128::zero())?;
    CLAIMED.save(deps.storage, &Uint128::zero())?;
    TOTAL_BONDED.save(deps.storage, &Uint128::zero())?;
    TOTAL_CLAIMED.save(deps.storage, &Uint128::zero())?;
    NUMBER_VALIDATORS.save(deps.storage, &Uint64::zero())?;

    Ok(Response::default())   
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Bond {nft_id} => execute_bond(deps, env, info, nft_id),
        ExecuteMsg::Unbond { nft_id, amount } => execute_unbond(deps, env, info, nft_id, amount),
        ExecuteMsg::Claim {nft_id, sender} => execute_claim(deps, env, info, nft_id, sender),
        ExecuteMsg::AddValidator { address, bond_denom, unbonding_period } => execute_add_validator (deps, env, info, address, bond_denom, unbonding_period),
        ExecuteMsg::RemoveValidator { address } => execute_remove_validator (deps, env, info, address, ),
        ExecuteMsg::BondCheck {} => execute_bond_check(deps.as_ref(), env, info),
        ExecuteMsg::CollectAngelRewards {  } => execute_collect_rewards(deps, env, info),
        ExecuteMsg::TransferBalanceToTreasury{  } => execute_transfer_balance(deps, env, info),
    }
}

pub fn execute_bond(deps: DepsMut, _env: Env, info: MessageInfo, nft_id: Uint128) -> Result<Response, ContractError> {
    let agent = AGENT.load(deps.storage)?;
    if info.sender != agent {
        return Err(ContractError::Unauthorized {});
    }
    // Making sure there is only one coin and handling the possible errors.
    let d_coins = match one_coin(&info) {
        Ok(coin) => coin,
        Err(err) => {
            match err {
                PaymentError::NoFunds{} => {return Err(ContractError::NoFunds {  });}
                PaymentError::MultipleDenoms{} => {return Err(ContractError::MultipleDenoms {  });}
                _ => {return Err(ContractError::InvalidCoin {  });}
            }
        },
    };
    let amount = d_coins.amount;

    let validator_address = chosen_validator(deps.as_ref(), None)?;

    // Update bonded tokens to validator
    let state = State::new();
    let mut validator_info = state.validator.load(deps.storage, &validator_address)?;
    validator_info.bonded.checked_add(amount.u128()).unwrap();
    state.validator.save(deps.storage, &validator_address, &validator_info)?;

    BONDED.update(deps.storage, |total| -> StdResult<_> {
            Ok(total.checked_add(amount)?)
    })?;

    TOTAL_BONDED.update(deps.storage, |total| -> StdResult<_> {
        Ok(total.checked_add(amount)?)
    })?;

    let res = Response::new()
        .add_message(StakingMsg::Delegate {
            validator: validator_address.to_string(),
            amount: d_coins,
        })
        .add_attribute("action", "bond")
        .add_attribute("from", nft_id)
        .add_attribute("bonded", amount)
        .add_attribute("validator", validator_address);
    Ok(res)
}


// Returns validator with the least amount of tokens bonded
// excluded address can not be returned 
pub fn chosen_validator (deps: Deps, excluded_address: Option<String>) -> Result<String, ContractError>  {
    let state = State::new();
    let validator_result : StdResult<Vec<_>>;
    if excluded_address.is_none() {
        validator_result = state.validator.idx.bonded
        .range(deps.storage,None,None,Order::Ascending)
        .take(1)
        .collect();
    } else {
        let excluded_address = excluded_address.unwrap();
        validator_result = state.validator.idx.bonded
        .range(deps.storage,None,None,Order::Ascending)
        .filter(|item| item.as_ref().unwrap().0 != excluded_address)
        .take(1)
        .collect();
    }

    let vec_validator_address = validator_result?;
    let validator_address = &vec_validator_address[0].0;

    Ok(validator_address.into())
}


pub fn execute_unbond(deps: DepsMut, env: Env, info: MessageInfo, nft_id: Uint128, amount: Uint128) -> Result<Response, ContractError> {
    let agent = AGENT.load(deps.storage)?;
    if info.sender != agent {
        return Err(ContractError::Unauthorized {});
    }
    // Returns the denomination that can be bonded (if there are multiple native tokens on the chain)
    let can_be_bonded_denom = deps.querier.query_bonded_denom()?;

    let total_number_validators = NUMBER_VALIDATORS.load(deps.storage)?;
    let number_validators= calc_validator_number(total_number_validators, amount)?;
    
    let vec_address_coin = chosen_validators_unstake(deps.as_ref(), amount, can_be_bonded_denom, number_validators)?;

    // Turn Vec<String, Coin> into Vec<StakingMsg>
    let msgs : Vec<StakingMsg> = vec_address_coin
    .clone()
    .into_iter()
    .map(|item| StakingMsg::Undelegate { validator: item.0, amount: item.1 })
    .collect();

    let state = State::new();
    for i in 0..vec_address_coin.len()-1 {
        // Remove from the validator info the required amount
        let mut validator_info = state.validator.load(deps.storage, &vec_address_coin[i].0)?;
        validator_info.bonded.checked_sub(vec_address_coin[i].1.amount.u128()).unwrap();
        state.validator.save(deps.storage,&vec_address_coin[i].0,&validator_info)?;

        CLAIMS.create_claim(
            deps.storage,
            &Addr::unchecked(nft_id.to_string()),
            vec_address_coin[i].1.amount,
            validator_info.unbonding_period.after(&env.block),  
        )?;
    }

    // If all validators have got the same unbonding_period. One single entry to CLAIMS could be done
    // CLAIMS.create_claim(
    //     deps.storage,
    //     &Addr::unchecked(nft_id.to_string()),
    //     amount,
    //     unbonding_period.after(&env.block),  
    // )?;


    BONDED.update(deps.storage, |total| -> StdResult<_> {
        Ok(total.checked_sub(amount)?)
    })?;   

    CLAIMED.update(deps.storage, |total| -> StdResult<_> {
        Ok(total.checked_add(amount)?)
    })?;  
    
    TOTAL_CLAIMED.update(deps.storage, |total| -> StdResult<_> {
        Ok(total.checked_add(amount)?)
    })?;  

    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "unbond")
        .add_attribute("from", nft_id)
        .add_attribute("unbonded", amount);
    Ok(res)
}


// It returns a vector with (validator_address, Coin) with information about the unstake about to happen. 
// PLAN_A: amount is split between the first 'number_validator' with more coin 'bonded'....
// PLAN_B: validators ordered Descending by bonded. Start unbonding all the coins from the first until we get 'amount'
// Confirms that the sum of the split_amount from selected validators is equal to amount
pub fn chosen_validators_unstake (deps: Deps, amount:Uint128, denom:String, number_validators: u64) -> Result<Vec<(String, Coin)>, ContractError>  {
    let limit = number_validators as usize;
    let amount_to_split = amount / Uint128::from(number_validators);
    let state = State::new();
    
    // Validators/Coin(amount, denom) from which we are going to unstake as vector<addr,Coin>
    let plana_validator_result : StdResult<Vec<(String, Coin)>> = state.validator.idx.bonded
    .range(deps.storage,None,None,Order::Descending)
    .filter(|item| 
        item.as_ref().unwrap().1.bonded > amount_to_split.u128() && item.as_ref().unwrap().1.bond_denom == denom)
    .map(|item|
        Ok((item.unwrap().0, coin(amount_to_split.u128(), &denom))))
    .take(limit)
    .collect();

    let count = plana_validator_result.as_ref().unwrap().len();

    let vec_address_coin:Vec<(String, Coin)> = if count != limit {
        let all_validators : StdResult<Vec<(String, Coin)>> = state.validator.idx.bonded
        .range(deps.storage,None,None,Order::Descending)
        .map(|item|
                Ok((item.unwrap().0, coin(amount_to_split.u128(), &denom)))
            )
        .collect();

        let vec_all_validators = all_validators?;

        let mut remaining_amount = amount.clone();
        let total_number_validators = vec_all_validators.len();
        let total_number_validators_u64 = total_number_validators as u64;
        let mut i = 0;
        let mut vec_planb_validator : Vec<(String, Coin)> = vec![];

        while remaining_amount > Uint128::zero() {

            if i > total_number_validators - 1 {
                return Err(ContractError::UnableUnstakeAmount {
                    amount: amount, number_validators: Uint64::from(total_number_validators_u64)
                });
            }

            let address = &vec_all_validators[i].0;
            let denom = &vec_all_validators[i].1.denom;
            let validator_amount = &vec_all_validators[i].1.amount.u128();

            if remaining_amount > vec_all_validators[i].1.amount {
                vec_planb_validator.push((address.to_string(),coin(*validator_amount, denom)));
                remaining_amount = remaining_amount - vec_all_validators[i].1.amount;
                i +=1;
            } else {
                vec_planb_validator.push((address.to_string(),coin(remaining_amount.u128(), denom)));
                break;
            }
        }
        vec_planb_validator
    } else {
        plana_validator_result?
    };

    let sum : u128 = vec_address_coin
    .iter()
    .map(|item| item.1.amount.u128())
    .sum();

    // Confirm the vector takes into account exactly the amount required
    if sum != amount.u128() {
        return Err(ContractError::UnableUnstakeAmount {
            amount: amount, number_validators: Uint64::from(number_validators)
        });
    }

     Ok(vec_address_coin)
}


// Calculates how many validators are going to be unstaken from. 
// At least one token has to be unstaked per validator 
pub fn calc_validator_number(number_validators: Uint64, amount: Uint128) -> StdResult<u64> {
    // Possible number of validators to split the bond is defined by the next vector. 
    // Powers of two, five or product of both to avoid repeating decimals on the amount to split between validators
    let v = vec![1, 2, 4, 5, 8, 10];  // 16, 20, 25, 32, 40, 50, 64, 80, 100

    let mut i = v.len();
    while i>1 {
        // At least one token to unbond per validator
        if v[i] <= number_validators.u64() && amount > Uint128::from(number_validators){
            return Ok(v[i]);
        }
        i= i.checked_sub(1).unwrap();
    }

    Ok(1)
}

pub fn execute_claim(deps: DepsMut, env: Env, info: MessageInfo, nft_id: Uint128, sender: String) -> Result<Response, ContractError> {
    let agent = AGENT.load(deps.storage)?;
    if info.sender != agent {
        return Err(ContractError::Unauthorized {});
    }

    let sender = deps.api.addr_validate(&sender)?;
    let can_be_bonded_denom = deps.querier.query_bonded_denom()?;
    let mut balance = deps
        .querier
        .query_balance(&env.contract.address, &can_be_bonded_denom)?;

    let to_send =
        CLAIMS.claim_tokens(deps.storage, &Addr::unchecked(nft_id), &env.block, None)?;

    if to_send == Uint128::zero() {
        return Err(ContractError::NothingToClaim {});
    }

    if balance.amount < to_send {
        return Err(ContractError::BalanceTooSmall {});
    }

    CLAIMED.update(deps.storage, |total| -> StdResult<_> {
        Ok(total.checked_sub(to_send)?)
    })?;  

    // transfer tokens to the sender
    balance.amount = to_send;
    let res = Response::new()
        .add_message(BankMsg::Send {
            to_address: sender.to_string(),
            amount: vec![balance],
        })
        .add_attribute("action", "claim")
        .add_attribute("from", sender)
        .add_attribute("nft_id", nft_id.to_string())
        .add_attribute("amount", to_send);
    Ok(res)
}

pub fn execute_add_validator(deps: DepsMut, _env: Env, info: MessageInfo, validator_address: String, bond_denom: String, unbonding_period: Duration) -> Result<Response, ContractError> {
    let manager = MANAGER.load(deps.storage)?;
    if info.sender != manager {
        return Err(ContractError::Unauthorized {});
    }
    // ensure the validator is registered
    let vals = deps.querier.query_all_validators()?;
    if !vals.iter().any(|v| v.address == validator_address) {
        return Err(ContractError::NotInValidatorSet {
            validator: validator_address,
        });
    }

    let state = State::new();
    if state.validator.has(deps.storage, &validator_address) {
        return Err(ContractError::ValidatorAlreadyRegistered{
            validator: validator_address,
        });
    }

    // Returns the denomination that can be bonded (if there are multiple native tokens on the chain)
    let can_be_bonded_denom = deps.querier.query_bonded_denom()?;

    if can_be_bonded_denom != bond_denom {
        return Err(ContractError::DenominationCanNotBeBonded{
            denom: bond_denom,
        });
    }

    let validator_info = ValidatorInfo{ 
        bond_denom, 
        unbonding_period,
        bonded: 0u128,
        claimed: 0u128,
    };

    state.validator.save(deps.storage, &validator_address, &validator_info)?;

    NUMBER_VALIDATORS.update(deps.storage, |total| -> StdResult<_> {
        Ok(total.checked_add(Uint64::from(1u64))?)
    })?;

    Ok(Response::default()
    .add_attribute("action", "add_validator")
    .add_attribute("validator_address", validator_address))
}

// Removes a validator. If it has got tokens staked, it redelegates them. If it has not delegated tokens, just removes it from state.
pub fn execute_remove_validator(deps: DepsMut, env: Env, info: MessageInfo, src_validator_address: String) -> Result<Response, ContractError> {
    let manager = MANAGER.load(deps.storage)?;
    if info.sender != manager {
        return Err(ContractError::Unauthorized {});
    }

    let state = State::new();

    if !state.validator.has(deps.storage, &src_validator_address) {
        return Err(ContractError::NotRegisteredValidator { address:src_validator_address });
    }

     let validator_count : u128 = state.validator.idx.bonded
    .range(deps.storage, None, None, Order::Descending)
    .into_iter()
    .count().try_into().unwrap();

    let option_full_delegation = deps.querier.query_delegation(env.contract.address,src_validator_address.clone())?;
    // What if the chosen validator is the one we are trying to remove??
    let dst_validator_address = chosen_validator(deps.as_ref(), Some(src_validator_address.clone()))?;

    state.validator.remove(deps.storage, &src_validator_address)?;
    let res:Response;
    if option_full_delegation.is_some() && validator_count ==1 {
        return Err(ContractError::CustomError { val: "Only one validator registered. Its delegations can not be redelegated".to_string() })
    } else if option_full_delegation.is_some(){
        let amount = option_full_delegation.unwrap().amount;
        // When we redelegate, by default all the pending rewards are claimed.
        let msg = StakingMsg::Redelegate { 
            src_validator:src_validator_address.to_string(), 
            dst_validator: dst_validator_address.clone(), 
            amount: amount.clone() 
        };

        res = Response::new()
        .add_message(msg)
        .add_attribute("action", "remove_validator")
        .add_attribute("address",src_validator_address)
        .add_attribute("redelegated_validator", dst_validator_address)
        .add_attribute("redelegated_denom", amount.denom)
        .add_attribute("redelegated_amount", amount.amount);
    } else {
        res = Response::new()
        .add_attribute("action", "remove_validator")
        .add_attribute("address",src_validator_address)
    }
     Ok(res)
}

// Check if chain delegated tokens by this contract match the value registered in TOTAL_BONDED state
pub fn execute_bond_check (deps: Deps, env:Env, info: MessageInfo) -> Result<Response, ContractError>{
    let manager = MANAGER.load(deps.storage)?;
    if info.sender != manager {
        return Err(ContractError::Unauthorized {});
    }

    // total number of tokensdelegated from this address
    // Expecting all delegations to be of the same denom
    let total_bonded = get_all_bonded(&deps.querier, &env.contract.address)?;

    let state_total_bonded = BONDED.load(deps.storage)?;
    if total_bonded != state_total_bonded {
        return Err(ContractError::BondedDiffer {
            total_bonded: total_bonded, state_total_bonded: state_total_bonded
        });       
    } 
    Ok(Response::default())
}

// get_bonded returns the total amount of delegations from contract to all validators
// it ensures they are all the same denom
fn get_all_bonded(querier: &QuerierWrapper, contract: &Addr) -> Result<Uint128, ContractError> {
    let bonds = querier.query_all_delegations(contract)?;
    if bonds.is_empty() {
        return Ok(Uint128::zero());
    }
    let denom = bonds[0].amount.denom.as_str();
    bonds.iter().fold(Ok(Uint128::zero()), |racc, d| {
        let acc = racc?;
        if d.amount.denom.as_str() != denom {
            Err(ContractError::DifferentBondDenom {
                denom1: denom.into(),
                denom2: d.amount.denom.to_string(),
            })
        } else {
            Ok(acc + d.amount.amount)
        }
    })
}

// Collect pending rewards from all validators
fn execute_collect_rewards ( deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError>{
    let manager = MANAGER.load(deps.storage)?;
    if info.sender != manager {
        return Err(ContractError::Unauthorized {});
    }
    // Any validator rewards have been previosly and automatically claimed when 'bonded change' occurred on any registered validator
    let state = State::new();
    let msgs : StdResult<Vec<DistributionMsg>> = state.validator.idx
        .bonded
        .range(deps.storage,None, None, Order::Descending)
        .filter(|item|
            item.as_ref().unwrap().1.bonded > 0)
        .map(|item| 
            Ok(DistributionMsg::WithdrawDelegatorReward { validator: item.unwrap().0 }))
        .collect();

    let msgs = msgs?;
    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "withdraw_delegation_rewards");
    Ok(res)
}

fn execute_transfer_balance (deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError>{
    let manager = MANAGER.load(deps.storage)?;
    if info.sender != manager {
        return Err(ContractError::Unauthorized {});
    }
    let balance = deps.querier.query_balance(&env.contract.address, deps.querier.query_bonded_denom()?)?;

    if balance.amount == Uint128::zero() {
        return Err(ContractError::CustomError { val: "Nothing to transfer. Amount for bonded denom is zero".to_string() })
    }

    let address = TREASURY.load(deps.storage)?;
    let msg = BankMsg::Send { to_address: address.clone(), amount: vec![balance.clone()] };

    Ok(Response::new()
    .add_message(CosmosMsg::Bank(msg))
    .add_attribute("action", "transfer_balance")
    .add_attribute("dst_addr", address)
    .add_attribute("denom", balance.denom)
    .add_attribute("amount", balance.amount)
    )

}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let state = State::new();
    match msg {
        // Returns #[returns(ClaimsResponse)]
        QueryMsg::Claims { nft_id } => {to_binary(&CLAIMS.query_claims(deps, &Addr::unchecked(nft_id))?)},
        // [returns(Validator_Info)]
        QueryMsg::ValidatorInfo {address} => to_binary(&state.validator.load(deps.storage,&address)?),
        // [returns(Validator_Deposits)]
        QueryMsg::TotalBonded {} => to_binary(&TOTAL_BONDED.may_load(deps.storage)?.unwrap_or_default()),
        QueryMsg::TotalClaimed{} => to_binary(&TOTAL_CLAIMED.may_load(deps.storage)?.unwrap_or_default()),
        QueryMsg::ContractBonded {} => to_binary(&BONDED.may_load(deps.storage)?.unwrap_or_default()),
        QueryMsg::ContractClaimed{} => to_binary(&CLAIMED.may_load(deps.storage)?.unwrap_or_default()),
        QueryMsg::BondedOnValidator{address} => to_binary(&query_bonded_on_validator(deps, env, address)?),
        QueryMsg::Agent{} => to_binary(&AGENT.load(deps.storage)?),
        QueryMsg::Manager{} => to_binary(&MANAGER.load(deps.storage)?),
        QueryMsg::RewardsBalance {  } => to_binary(&deps.querier.query_balance(&env.contract.address, deps.querier.query_bonded_denom()?)?),
    }
}

pub fn query_bonded_on_validator(deps: Deps, env: Env,  val_address:String) -> StdResult<Uint128> {
     let bonded = bonded_on_validator(&deps.querier, &env.contract.address, &deps.api.addr_validate(&val_address)?).unwrap();
    Ok(bonded)
}

// get_bonded returns the total amount of delegations from contract to a certain validator
// Not in use at the moment.
fn bonded_on_validator(querier: &QuerierWrapper, delegator: &Addr, validator: &Addr) -> Result<Uint128, ContractError> {
    let option_full_delegation = querier.query_delegation(delegator,validator)?;
    if option_full_delegation.is_none() {
        return Ok(Uint128::zero());
    }
    let full_delegation = option_full_delegation.unwrap(); //.amount.denom.as_str();
    let _denom = full_delegation.amount.denom.as_str();
    let amount = full_delegation.amount.amount;

    Ok(Uint128::from(amount))
}

// *****************************************************************************************************************************
// *****************************************************************************************************************************
// *****************************************************************************************************************************
// *****************************************************************************************************************************
#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockQuerier, MOCK_CONTRACT_ADDR,
    };
    use cosmwasm_std::{
        coins, Coin, CosmosMsg, Decimal, FullDelegation, OverflowError, OverflowOperation,
        Validator,
    };
    use cw_controllers::Claim;
    use cw_utils::{Duration, DAY, HOUR, WEEK};

    const MANAGER: &str = "manager";
    const AGENT: &str = "agent";
    const TREASURY: &str = "treasury";

    const DELEGATOR1: &str = "bob";
    const JANE: &str = "jane";
    const BRUCE: &str = "bruce";

    const VALIDATOR1: &str = "validator1";
    const VALIDATOR2: &str = "validator2";
    const VALIDATOR3: &str = "validator3";



    fn sample_validator(addr: &str) -> Validator {
        Validator {
            address: addr.into(),
            commission: Decimal::percent(3),
            max_commission: Decimal::percent(10),
            max_change_rate: Decimal::percent(1),
        }
    }

    fn sample_delegation(val_addr: &str, amount: Coin) -> FullDelegation {
        let can_redelegate = amount.clone();
        let accumulated_rewards = coins(0, &amount.denom);
        FullDelegation {
            validator: val_addr.into(),
            delegator: Addr::unchecked(MOCK_CONTRACT_ADDR),
            amount,
            can_redelegate,
            accumulated_rewards,
        }
    }

    fn set_validator(querier: &mut MockQuerier) {
        querier.update_staking("ustake", &[sample_validator(VALIDATOR1)], &[]);
    }

    fn set_validators(querier: &mut MockQuerier) {
        querier.update_staking("ustake", &[sample_validator(VALIDATOR1), sample_validator(VALIDATOR2), sample_validator(VALIDATOR3)], &[]);
    }

    fn set_delegation(querier: &mut MockQuerier, amount: u128, denom: &str) {
        querier.update_staking(
            "ustake",
            &[sample_validator(VALIDATOR1)],
            &[sample_delegation(VALIDATOR1, coin(amount, denom))],
        );
    }

    // just a test helper, forgive the panic
    fn later(env: &Env, delta: Duration) -> Env {
        let time_delta = match delta {
            Duration::Time(t) => t,
            _ => panic!("Must provide duration in time"),
        };
        let mut res = env.clone();
        res.block.time = res.block.time.plus_seconds(time_delta);
        res
    }

    fn get_claims(deps: Deps, addr: &str) -> Vec<Claim> {
        CLAIMS
            .query_claims(deps, &Addr::unchecked(addr))
            .unwrap()
            .claims
    }

    fn default_instantiate(tax_percent: u64, min_withdrawal: u128) -> InstantiateMsg {
        InstantiateMsg {
            agent: AGENT.into(),
            manager: MANAGER.into(),
            treasury: TREASURY.into(),
        }
    }

    #[test]
    fn mint() {
        let mut deps = mock_dependencies();
        //let contract: Cw721Contract<Extension, Empty> = cw721_base::Cw721Contract::default();

        let info = mock_info(AGENT, &[]);
        // let init_msg = InstantiateMsg {

        // };
//        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
    }


}