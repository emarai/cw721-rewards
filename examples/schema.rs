use cosmwasm_schema::write_api;
use cosmwasm_std::Empty;

use cw721_rewards::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<Empty>,
        query: QueryMsg<Empty>,
    }
}
