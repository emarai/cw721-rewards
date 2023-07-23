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

use cosmwasm_schema::cw_serde;
pub use cw_ownable::{Action, Ownership, OwnershipError};

use cosmwasm_std::Empty;

// Version info for migration
pub const CONTRACT_NAME: &str = "crates.io:cw721-rewards";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REWARDS_WITHDRAW_REPLY: u64 = 1001;

pub use archway_bindings::types::rewards::WithdrawRewardsResponse;
pub use archway_bindings::{ArchwayMsg, ArchwayQuery};
pub use cosmwasm_std::{DepsMut, Reply, Response, StdError, StdResult};
pub use cw_utils::NativeBalance;

#[cw_serde]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

#[cw_serde]
#[derive(Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
    pub royalty_percentage: Option<u64>,
    pub royalty_payment_address: Option<String>,
}

pub type Extension = Option<Metadata>;

pub mod entry {
    use crate::msg::{Cw2981QueryMsg, MigrateMsg};

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

        if let ExecuteMsg::Mint {
            extension:
                Some(Metadata {
                    royalty_percentage: Some(royalty_percentage),
                    ..
                }),
            ..
        } = &msg
        {
            // validate royalty_percentage to be between 0 and 100
            // no need to check < 0 because royalty_percentage is u64
            if *royalty_percentage > 100 {
                return Err(ContractError::InvalidRoyaltyPercentage);
            }
        }
        tract.execute(deps, env, info, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg<Cw2981QueryMsg>) -> StdResult<Binary> {
        let tract = Cw721Contract::<Extension, Empty, Empty, Cw2981QueryMsg>::default();
        tract.query(deps, env, msg)
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
        match msg.id {
            REWARDS_WITHDRAW_REPLY => rewards::after_rewards_withdrawn(deps, msg),
            id => Err(StdError::not_found(format!("Unknown reply id: {}", id))),
        }
    }

    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
        Ok(Response::default())
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
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Uint128,
    };
    use cw2::ContractVersion;
    use cw721::Cw721Query;

    const CREATOR: &str = "creator";

    use crate::msg::{CheckRoyaltiesResponse, Cw2981QueryMsg, RoyaltiesInfoResponse};

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
                minter: "larry".into(),
                rewards_denom: "aconst".into(),
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

    #[test]
    fn use_metadata_extension() {
        let mut deps = mock_dependencies();
        let contract = Cw721Contract::<Extension, Empty, Empty, Cw2981QueryMsg>::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
            rewards_denom: "aconst".to_string(),
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
        let extension = Some(Metadata {
            description: Some("Spaceship with Warp Drive".into()),
            name: Some("Starship USS Enterprise".to_string()),
            ..Metadata::default()
        });
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: token_uri.clone(),
            extension: extension.clone(),
        };
        entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.token_uri, token_uri);
        assert_eq!(res.extension, extension);
    }

    #[test]
    fn validate_royalty_information() {
        let mut deps = mock_dependencies();
        let _contract = Cw721Contract::<Extension, Empty, Empty, Cw2981QueryMsg>::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
            rewards_denom: "aconst".to_string(),
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Enterprise".to_string()),
                royalty_percentage: Some(101),
                ..Metadata::default()
            }),
        };
        // mint will return StdError
        let err = entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidRoyaltyPercentage);
    }

    #[test]
    fn check_royalties_response() {
        let mut deps = mock_dependencies();
        let contract = Cw721Contract::<Extension, Empty, Empty, Cw2981QueryMsg>::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
            rewards_denom: "aconst".to_string(),
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Enterprise".to_string()),
                ..Metadata::default()
            }),
        };
        entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let expected = CheckRoyaltiesResponse {
            royalty_payments: true,
        };
        let res = contract.check_royalties(deps.as_ref()).unwrap();
        assert_eq!(res, expected);

        // also check the longhand way
        let query_msg = QueryMsg::Extension {
            msg: Cw2981QueryMsg::CheckRoyalties {},
        };
        let query_res: CheckRoyaltiesResponse =
            from_binary(&entry::query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(query_res, expected);
    }

    #[test]
    fn check_token_royalties() {
        let mut deps = mock_dependencies();

        let contract = Cw721Contract::<Extension, Empty, Empty, Cw2981QueryMsg>::default();
        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
            rewards_denom: "aconst".to_string(),
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let owner = "jeanluc";
        let exec_msg = ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: owner.into(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Enterprise".to_string()),
                royalty_payment_address: Some("jeanluc".to_string()),
                royalty_percentage: Some(10),
                ..Metadata::default()
            }),
        };
        entry::execute(deps.as_mut(), mock_env(), info.clone(), exec_msg).unwrap();

        let expected = RoyaltiesInfoResponse {
            address: owner.into(),
            royalty_amount: Uint128::new(10),
        };
        let res = contract
            .query_royalties_info(deps.as_ref(), token_id.to_string(), Uint128::new(100))
            .unwrap();
        assert_eq!(res, expected);

        // also check the longhand way
        let query_msg = QueryMsg::Extension {
            msg: Cw2981QueryMsg::RoyaltyInfo {
                token_id: token_id.to_string(),
                sale_price: Uint128::new(100),
            },
        };
        let query_res: RoyaltiesInfoResponse =
            from_binary(&entry::query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(query_res, expected);

        // check for rounding down
        // which is the default behaviour
        let voyager_token_id = "Voyager";
        let owner = "janeway";
        let voyager_exec_msg = ExecuteMsg::Mint {
            token_id: voyager_token_id.to_string(),
            owner: owner.into(),
            token_uri: Some("https://starships.example.com/Starship/Voyager.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Voyager".to_string()),
                royalty_payment_address: Some("janeway".to_string()),
                royalty_percentage: Some(4),
                ..Metadata::default()
            }),
        };
        entry::execute(deps.as_mut(), mock_env(), info, voyager_exec_msg).unwrap();

        // 43 x 0.04 (i.e., 4%) should be 1.72
        // we expect this to be rounded down to 1
        let voyager_expected = RoyaltiesInfoResponse {
            address: owner.into(),
            royalty_amount: Uint128::new(1),
        };

        let res = contract
            .query_royalties_info(
                deps.as_ref(),
                voyager_token_id.to_string(),
                Uint128::new(43),
            )
            .unwrap();
        assert_eq!(res, voyager_expected);
    }
}
