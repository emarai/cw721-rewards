mod error;
mod execute;
pub mod helpers;
pub mod msg;
mod query;
pub mod state;

#[cfg(test)]
mod contract_tests;

pub use crate::error::ContractError;
pub use crate::msg::{ExecuteMsg, InstantiateMsg, MinterResponse, QueryMsg};
pub use crate::state::Cw721Contract;

// These types are re-exported so that contracts interacting with this
// one don't need a direct dependency on cw_ownable to use the API.
//
// `Action` is used in `ExecuteMsg::UpdateOwnership`, `Ownership` is
// used in `QueryMsg::Ownership`, and `OwnershipError` is used in
// `ContractError::Ownership`.
pub use cw_ownable::{Action, Ownership, OwnershipError};

use cosmwasm_std::Empty;

// This is a simple type to let us handle empty extensions
pub type Extension = Option<Empty>;

// Version info for migration
pub const CONTRACT_NAME: &str = "crates.io:cw721-rewards";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DENOM: &str = "uconst";

const REWARDS_WITHDRAW_REPLY: u64 = 1001;

pub use archway_bindings::types::rewards::WithdrawRewardsResponse;
pub use archway_bindings::{ArchwayMsg, ArchwayQuery};
pub use cosmwasm_std::{DepsMut, Reply, Response, StdError, StdResult};
pub use cw_utils::NativeBalance;

pub mod entry {
    use super::*;

    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{
        Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
    };

    // This makes a conscious choice on the various generics used by the contract
    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        let tract = Cw721Contract::<Extension, Empty, Empty, Empty>::default();
        tract.instantiate(deps, env, info, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<Extension>,
    ) -> Result<Response<ArchwayMsg>, ContractError> {
        let tract = Cw721Contract::<Extension, ArchwayMsg, Empty, Empty>::default();
        tract.execute(deps, env, info, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg<Empty>) -> StdResult<Binary> {
        let tract = Cw721Contract::<Extension, Empty, Empty, Empty>::default();
        tract.query(deps, env, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
        match msg.id {
            REWARDS_WITHDRAW_REPLY => rewards::after_rewards_withdrawn(deps, msg),
            id => Err(StdError::not_found(format!("Unknown reply id: {}", id))),
        }
    }
}

pub mod rewards {

    use cosmwasm_std::{Binary, SubMsgResponse};

    use super::*;

    pub fn after_rewards_withdrawn(deps: DepsMut, msg: Reply) -> StdResult<Response> {
        let tract = Cw721Contract::<Extension, ArchwayMsg, Empty, Empty>::default();

        let data = parse_reply_data(msg)?;
        let withdraw_response: WithdrawRewardsResponse =
            serde_json_wasm::from_slice::<WithdrawRewardsResponse>(&data.0)
                .map_err(|e| StdError::generic_err(e.to_string()))?;

        let mut rewards_balance = NativeBalance(withdraw_response.total_rewards);
        rewards_balance.normalize();

        let total_rewards: Vec<String> = rewards_balance
            .clone()
            .into_vec()
            .iter()
            .map(|coin| coin.to_string())
            .collect();

        let total_rewards_u128: u128 = rewards_balance
            .into_vec()
            .iter()
            .map(|coin| coin.amount.u128())
            .sum();

        tract.add_total_arch_reward(deps.storage, total_rewards_u128)?;

        let res = Response::new()
            .add_attribute("method", "after_rewards_withdrawn")
            .add_attribute("records_num", withdraw_response.records_num.to_string())
            .add_attribute("total_rewards", total_rewards.concat());

        Ok(res)
    }

    fn parse_reply_data(reply: Reply) -> StdResult<Binary> {
        parse_reply_result(reply)?
            .data
            .ok_or_else(|| StdError::generic_err("Missing reply data".to_owned()))
    }

    fn parse_reply_result(reply: Reply) -> StdResult<SubMsgResponse> {
        reply.result.into_result().map_err(StdError::generic_err)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cw2::ContractVersion;

    use super::*;

    /// Make sure cw2 version info is properly initialized during instantiation.
    #[test]
    fn proper_cw2_initialization() {
        let mut deps = mock_dependencies();

        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("larry", &[]),
            InstantiateMsg {
                name: "".into(),
                symbol: "".into(),
                max_supply: 0,
                token_uri: "".into(),
            },
        )
        .unwrap();

        let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(
            version,
            ContractVersion {
                contract: CONTRACT_NAME.into(),
                version: CONTRACT_VERSION.into(),
            },
        );
    }
}
