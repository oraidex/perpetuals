use crate::ownership::OwnerProposal;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use margined_common::asset::AssetInfo;

pub const OWNER: Admin = Admin::new("owner");
pub const OWNERSHIP_PROPOSAL: Item<OwnerProposal> = Item::new("ownership_proposals");

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE: Item<State> = Item::new("state");

pub const TOTAL_STAKED: Item<Uint128> = Item::new("total_staked");
pub const REWARDS_PER_TOKEN: Item<Uint128> = Item::new("rewards_per_token");
pub const USER_STAKE: Map<Addr, UserStake> = Map::new("staked_amounts");

#[cw_serde]
pub struct Config {
    pub fee_pool: Addr,
    pub deposit_token: AssetInfo,
    pub reward_token: AssetInfo,
    pub tokens_per_interval: Uint128,
}

#[cw_serde]
pub struct State {
    pub is_open: bool,
    pub last_distribution: Timestamp,
}

#[cw_serde]
pub struct UserStake {
    pub staked_amounts: Uint128,
    pub claimable_rewards: Uint128,
    pub previous_cumulative_rewards_per_token: Uint128,
    pub cumulative_rewards: Uint128,
}

impl Default for UserStake {
    fn default() -> Self {
        Self {
            staked_amounts: Uint128::zero(),
            claimable_rewards: Uint128::zero(),
            previous_cumulative_rewards_per_token: Uint128::zero(),
            cumulative_rewards: Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct Pool {
    pub id: u64,
    pub quote_denom: String,
}
