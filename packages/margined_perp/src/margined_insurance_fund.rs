use cosmwasm_schema::{cw_serde, QueryResponses};
use margined_common::asset::AssetInfo;

use cosmwasm_std::{Addr, Decimal, Uint128};
#[cw_serde]
pub struct InstantiateMsg {
    pub engine: String,
    pub perp_token: String,
    pub additional_mint_rate: Decimal,
    pub smart_router: String,
    pub swap_router: String,
    pub swap_fee: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner {
        owner: String,
    },
    AddVamm {
        vamm: String,
    },
    RemoveVamm {
        vamm: String,
    },
    Withdraw {
        token: AssetInfo,
        amount: Uint128,
    },
    ShutdownVamms {},
    UpdateSwapInfo {
        smart_router: Option<String>,
        swap_router: Option<String>,
        swap_fee: Option<Decimal>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(OwnerResponse)]
    GetOwner {},
    #[returns(VammResponse)]
    IsVamm { vamm: String },
    #[returns(AllVammResponse)]
    GetAllVamm { limit: Option<u32> },
    #[returns(AllVammStatusResponse)]
    GetAllVammStatus { limit: Option<u32> },
    #[returns(VammStatusResponse)]
    GetVammStatus { vamm: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct ConfigResponse {
    pub engine: Addr,
    pub perp_token: Addr,
    pub additional_mint_rate: Decimal,
}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct VammResponse {
    pub is_vamm: bool,
}

#[cw_serde]
pub struct VammStatusResponse {
    pub vamm_status: bool,
}

#[cw_serde]
pub struct AllVammResponse {
    pub vamm_list: Vec<Addr>,
}

#[cw_serde]
pub struct AllVammStatusResponse {
    pub vamm_list_status: Vec<(Addr, bool)>,
}
