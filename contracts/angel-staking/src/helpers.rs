use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, CustomQuery, Querier, QuerierWrapper, StdResult, WasmMsg, WasmQuery,
};

use crate::msg::{ExecuteMsg, QueryMsg};


/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakingContract(pub Addr);

impl StakingContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;              
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }


    // /// Get Owner of an NFT
    // pub fn get_owner<Q, T, CQ>(&self, querier: &Q, token_id:String) -> StdResult<OwnerOfResponse>
    // where
    //     Q:Querier,
    //     T: Into<String>,
    //     CQ: CustomQuery,
    // {
    //     let msg: QueryMsg = QueryMsg::OwnerOf { token_id:token_id, include_expired:None };
    //     let query = WasmQuery::Smart { contract_addr: self.addr().into(), msg: to_binary(&msg)? }.into();
    //     let res: OwnerOfResponse = QuerierWrapper::<CQ>::new(querier).query(&query)?;
    //     Ok(res)
    // }

    // /// Get All Tokens
    // pub fn all_tokens<Q, T, CQ>(&self, querier: &Q) -> StdResult<TokensResponse>
    // where
    //     Q:Querier,
    //     T: Into<String>,
    //     CQ: CustomQuery,
    // {
    //     let msg = QueryMsg::AllTokens { start_after: None, limit: None };
    //     let query = WasmQuery::Smart { contract_addr: self.addr().into(), msg: to_binary(&msg)? }.into();
    //     let res: TokensResponse = QuerierWrapper::<CQ>::new(querier).query(&query)?;
    //     Ok(res)
    // }

    // /// Get NFT Info
    // pub fn get_nft_info<Q, T, CQ>(&self, querier: &Q, token_id:String) -> StdResult<NftInfoResponse<Metadata>>
    // where
    //     Q:Querier,
    //     T: Into<String>,
    //     CQ: CustomQuery,
    // {
    //     let msg: QueryMsg = QueryMsg::NftInfo { token_id: token_id };
    //     let query = WasmQuery::Smart { contract_addr: self.addr().into(), msg: to_binary(&msg)? }.into();
    //     let res: NftInfoResponse<_> = QuerierWrapper::<CQ>::new(querier).query(&query)?;
    //     Ok(res)
    // }
    
}
