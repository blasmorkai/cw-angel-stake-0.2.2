use cosmwasm_schema::write_api;

use nft::msg::{ExecuteMsg, QueryMsg};
use cw721_base::InstantiateMsg;

fn main() {
    write_api! {                                //3) <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<
       instantiate: InstantiateMsg,
       execute: ExecuteMsg,
       query: QueryMsg,
    }
}