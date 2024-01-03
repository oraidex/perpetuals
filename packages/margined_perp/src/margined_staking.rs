use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Timestamp, Uint128};
use margined_common::asset::AssetInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub fee_collector: String,
    pub deposit_token: AssetInfo,
    pub reward_token: AssetInfo,
    pub tokens_per_interval: Uint128,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        tokens_per_interval: Option<Uint128>,
    },
    UpdateRewards {},
    Stake {},
    Unstake {
        amount: Uint128,
    },
    Claim {
        recipient: Option<String>,
    },
    Pause {},
    Unpause {},
    ProposeNewOwner {
        new_owner: String,
        duration: u64,
    },
    RejectOwner {},
    ClaimOwnership {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(StateResponse)]
    State {},
    #[returns(Addr)]
    Owner {},
    #[returns(Uint128)]
    GetClaimable { user: String },
    #[returns(UserStakedResponse)]
    GetUserStakedAmount { user: String },
    #[returns(TotalStakedResponse)]
    GetTotalStakedAmount {},
    #[returns(OwnerProposalResponse)]
    GetOwnershipProposal {},
}

#[cw_serde]
pub struct TotalStakedResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct UserStakedResponse {
    pub staked_amounts: Uint128,
    pub claimable_rewards: Uint128,
    pub previous_cumulative_rewards_per_token: Uint128,
    pub cumulative_rewards: Uint128,
}

#[cw_serde]
pub struct ConfigResponse {
    pub fee_collector: Addr,
    pub deposit_token: AssetInfo,
    pub reward_token: AssetInfo,
    pub tokens_per_interval: Uint128,
}

#[cw_serde]
pub struct StateResponse {
    pub is_open: bool,
    pub last_distribution: Timestamp,
}

#[cw_serde]
pub struct OwnerProposalResponse {
    pub owner: Addr,
    pub expiry: u64,
}
