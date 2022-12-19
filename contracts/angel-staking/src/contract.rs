use core::num;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, Decimal, Deps, DepsMut, DistributionMsg, Env,
    MessageInfo, QuerierWrapper, Response, StakingMsg, StdError, StdResult, Uint128, WasmMsg, Uint64,
    Order, Coin,
};

use cw2::set_contract_version;
use cw_utils::{one_coin, PaymentError, Duration, Expiration};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg,  QueryMsg};
use crate::state::{VALIDATOR_DEPOSITS, VALIDATOR_INFO, TOTAL_BONDED, TOTAL_CLAIMED, AGENT, MANAGER, CLAIMS, Validator_Deposits, Validator_Info };

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
    
    AGENT.save(deps.storage, &msg.agent)?;
    MANAGER.save(deps.storage, &msg.manager)?;
    TOTAL_BONDED.save(deps.storage, &Uint128::zero())?;
    TOTAL_CLAIMED.save(deps.storage, &Uint128::zero())?;

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
        ExecuteMsg::Bond {nft_id, amount} => bond(deps, env, info, nft_id, amount),
        ExecuteMsg::Unbond { nft_id, amount } => unbond(deps, env, info, nft_id, amount),
        ExecuteMsg::Claim {nft_id} => claim(deps, env, info, nft_id),
        ExecuteMsg::AddValidator { address, bond_denom, unbonding_period } => add_validator (deps, env, info, address, bond_denom, unbonding_period),
        ExecuteMsg::RemoveValidator { address } => remove_validator (deps, env, info, address, ),
        ExecuteMsg::BondCheck {} => bond_check(deps.as_ref(), env),
    }
}

pub fn bond(deps: DepsMut, env: Env, info: MessageInfo, nft_id: Uint128, amount: Uint128) -> Result<Response, ContractError> {
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

    let validator_address = chosen_validator_stake(deps.as_ref())?;

    // Update bonded tokens to validator
    // QUESTION next line: rust analyzer says it does not need to be mutable??
    let mut validator_info = VALIDATOR_INFO.load(deps.storage, &validator_address)?;
    validator_info.total_bonded.checked_add(amount).unwrap();
    VALIDATOR_INFO.save(deps.storage, &validator_address, &validator_info)?;

            // VALIDATOR_INFO.update(deps.storage,&validator_address, |mut val| -> Result<_,ContractError> {
            //     val.unwrap().total_bonded.checked_add(amount);
            //     Ok(val.unwrap())
            // })?;

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

// TODO: implement some logic on which validator will be staking the tokens
// Now just returning one whatsoever
pub fn chosen_validator_stake (deps: Deps) -> Result<String, ContractError>  {
    let validator_result : StdResult<Vec<_>> = VALIDATOR_INFO
    .range(deps.storage,None,None,Order::Ascending)
    .take(1)
    .collect();

    let vec_validator_address = validator_result?;
    let validator_address = &vec_validator_address[0].0;

    Ok(validator_address.into())
}


pub fn unbond(deps: DepsMut, env: Env, info: MessageInfo, nft_id: Uint128, amount: Uint128) -> Result<Response, ContractError> {

    // Returns the denomination that can be bonded (if there are multiple native tokens on the chain)
    let can_be_bonded_denom = deps.querier.query_bonded_denom()?;

    // TODO: Before calling this function the number of validators has to calculated. A power of 2 or 5 or combination of them
    let number_validators=2;
    let amount_to_split = amount / Uint128::from(number_validators);
    let vec_address_coin = chosen_validators_unstake(deps.as_ref(), amount, amount_to_split, can_be_bonded_denom, number_validators)?;

    // Turn Vec<String, Coin> into Vec<StakingMsg>
    let msgs : Vec<StakingMsg> = vec_address_coin
    .clone()
    .into_iter()
    .map(|item| StakingMsg::Undelegate { validator: item.0, amount: item.1 })
    .collect();


    for i in 0..vec_address_coin.len()-1 {
        // Remove from the validator info the required amount
        let mut validator_info = VALIDATOR_INFO.load(deps.storage, &vec_address_coin[i].0)?;
        validator_info.total_bonded.checked_sub(vec_address_coin[i].1.amount).unwrap();
        VALIDATOR_INFO.save(deps.storage,&vec_address_coin[i].0,&validator_info)?;

        let expiration= validator_info.unbonding_period.after(&env.block);
        let expiration_cw20: cw20::Expiration = validator_info.unbonding_period.after(&env.block);

        //ERROR: the expiration field from create claim seems to come from cw20 and clashes with cw-utils implementation.
        //ERROR Description: expected enum `cw20::Expiration`, found enum `cw_utils::Expiration`
        //TODO, Should one claim per validator be created, or just a simple one for everything, outside the loop.
        CLAIMS.create_claim(
            deps.storage,
            &Addr::unchecked(nft_id.to_string()),
            vec_address_coin[i].1.amount,
            cw20::Expiration::AtHeight(20u64),  // <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
        )?;

    }

    TOTAL_BONDED.update(deps.storage, |total| -> StdResult<_> {
        Ok(total.checked_sub(amount)?)
    })?;   

    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "unbond")
        .add_attribute("from", nft_id)
        .add_attribute("unbonded", amount);
    Ok(res)
}


// It returns a vector with (validator_address, Coin) .
// Confirms that the sum of the split_amount from selected validators is equal to amount
pub fn chosen_validators_unstake (deps: Deps, amount:Uint128, amount_to_split:Uint128, denom:String, number_validators: u64) -> Result<Vec<(String, Coin)>, ContractError>  {
    let limit = number_validators as usize;

    // Validators/Coin(amount, denom) from which we are going to unstake as vector<addr,Coin>
    let validator_result : StdResult<Vec<(String, Coin)>> = VALIDATOR_INFO
    .range(deps.storage,None,None,Order::Ascending)
    .filter(|item| 
        item.as_ref().unwrap().1.total_bonded > amount_to_split && item.as_ref().unwrap().1.bond_denom == denom)
    .map(|item|
        Ok((item.unwrap().0, coin(amount_to_split.u128(), &denom))))
    .take(limit)
    .collect();

    // Sum of all amounts from previous vector
    let sum : u128 = validator_result.as_ref()
    .unwrap()
    .iter()
    .map(|item| item.1.amount.u128())
    .sum();

    // Confirm the vector takes account exactly of the amount required
    if sum != amount.u128() {
        return Err(ContractError::UnableUnstakeAmount {
            amount: amount, number_validators: Uint64::from(number_validators)
        });
    }

    let vec_address_coin:Vec<(String, Coin)> = validator_result?;


     Ok(vec_address_coin)
}

pub fn claim(deps: DepsMut, env: Env, info: MessageInfo, nft_id: Uint128) -> Result<Response, ContractError> {
    unimplemented!()
}

pub fn add_validator(deps: DepsMut, env: Env, info: MessageInfo, validator_address: String, bond_denom: String, unbonding_period: Duration) -> Result<Response, ContractError> {
    // ensure the validator is registered
    let vals = deps.querier.query_all_validators()?;
    if !vals.iter().any(|v| v.address == validator_address) {
        return Err(ContractError::NotInValidatorSet {
            validator: validator_address,
        });
    }

    if VALIDATOR_INFO.has(deps.storage, &validator_address) {
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

    let validator_info = Validator_Info{ 
        address: validator_address.clone(), 
        bond_denom, 
        unbonding_period,
        total_bonded: Uint128::zero(),
        min_withdraw: Uint64::from(1u32),
    };

    VALIDATOR_INFO.save(deps.storage, &validator_address, &validator_info)?;

    Ok(Response::default()
    .add_attribute("action", "add_validator")
    .add_attribute("validator_address", validator_address))
}


pub fn remove_validator(deps: DepsMut, env: Env, info: MessageInfo, address: String) -> Result<Response, ContractError> {
    unimplemented!()
}

// Check if chain delegated tokens by this contract match the value registered in TOTAL_BONDED state
pub fn bond_check (deps: Deps, env:Env) -> Result<Response, ContractError>{
    // total number of tokensdelegated from this address
    // Expecting all delegations to be of the same denom
    let total_bonded = get_all_bonded(&deps.querier, &env.contract.address)?;

    let state_total_bonded = TOTAL_BONDED.load(deps.storage)?;
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

// get_bonded returns the total amount of delegations from contract to a certain validator
fn get_bonded(querier: &QuerierWrapper, delegator: &Addr, validator: &Addr) -> Result<Uint128, ContractError> {
    let option_full_delegation = querier.query_delegation(delegator,validator)?;
    if option_full_delegation.is_none() {
        return Ok(Uint128::zero());
    }
    let full_delegation = option_full_delegation.unwrap(); //.amount.denom.as_str();
    let _denom = full_delegation.amount.denom.as_str();
    let amount = full_delegation.amount.amount;

    Ok(Uint128::from(amount))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Returns #[returns(ClaimsResponse)]
        QueryMsg::Claims { nft_id } => {to_binary(&CLAIMS.query_claims(deps, &deps.api.addr_validate(&nft_id)?)?)},
        // [returns(Validator_Info)]
        QueryMsg::ValidatorInfo {address} => to_binary(&VALIDATOR_INFO.load(deps.storage,&address)?),
        // [returns(Validator_Deposits)]
        QueryMsg::ValidatorDeposits {address} => to_binary(&VALIDATOR_DEPOSITS.load(deps.storage,&address)?),
        QueryMsg::TotalBonded {} => to_binary(&TOTAL_BONDED.may_load(deps.storage)?.unwrap_or_default()),
        QueryMsg::TotalClaimed{} => to_binary(&TOTAL_CLAIMED.may_load(deps.storage)?.unwrap_or_default()),
        QueryMsg::Agent{} => to_binary(&AGENT.load(deps.storage)?),
        QueryMsg::Manager{} => to_binary(&MANAGER.load(deps.storage)?),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, coins, from_binary};

    const CREATOR: &str = "creator";

    #[test]
    fn mint() {
        let mut deps = mock_dependencies();
        //let contract: Cw721Contract<Extension, Empty> = cw721_base::Cw721Contract::default();

        let info = mock_info(CREATOR, &[]);
        // let init_msg = InstantiateMsg {

        // };
//        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

 

    }


}