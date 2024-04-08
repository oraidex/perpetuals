use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin, to_binary, Addr, CosmosMsg, Decimal, QuerierWrapper, StdError, StdResult, Uint128,
    WasmMsg,
};

use cw20::Cw20ExecuteMsg;
use margined_common::asset::AssetInfo;

const OFFER_AMOUNT_DEFAULT: u128 = 1000000;

#[cw_serde]
pub struct SwapInfoResponse {
    pub swap_router: Addr,
    pub smart_router: Addr,
    pub swap_fee: Decimal,
}

#[cw_serde]
pub struct SmartRouterController(pub String);

#[cw_serde]
pub struct GetSmartRouteResponse {
    pub swap_ops: Vec<SwapOperation>,
    pub actual_minimum_receive: Uint128,
}

#[cw_serde]
pub enum SmartRouterQueryMsg {
    GetSmartRoute {
        input_info: AssetInfo,
        output_info: AssetInfo,
        offer_amount: Uint128,
    },
}

#[cw_serde]
pub enum SwapOperation {
    // swap cw20 token
    OraiSwap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}

#[cw_serde]
pub enum SwapRouterExecuteMsg {
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<Addr>,
    },
}

impl SmartRouterController {
    pub fn addr(&self) -> String {
        self.0.clone()
    }

    pub fn build_swap_operations(
        &self,
        querier: &QuerierWrapper,
        offer_asset: AssetInfo,
        ask_asset: AssetInfo,
        amount: Option<Uint128>,
    ) -> StdResult<GetSmartRouteResponse> {
        let offer_amount = amount.unwrap_or(Uint128::from(OFFER_AMOUNT_DEFAULT));

        match querier.query_wasm_smart::<GetSmartRouteResponse>(
            self.addr(),
            &SmartRouterQueryMsg::GetSmartRoute {
                input_info: offer_asset.clone(),
                output_info: ask_asset.clone(),
                offer_amount: offer_amount.clone(),
            },
        ) {
            Ok(val) => return Ok(val),
            Err(err) => {
                return Err(StdError::generic_err(format!(
                    "Cannot simulate swap with ops: with error: {:?}",
                    err.to_string()
                )));
            }
        };
    }

    pub fn simulate_belief_price(
        &self,
        querier: &QuerierWrapper,
        offer_asset: AssetInfo,
        ask_asset: AssetInfo,
        swap_fee: Decimal,
    ) -> StdResult<Decimal> {
        let simulate = self.build_swap_operations(querier, offer_asset, ask_asset, None)?;
        let mut belief_price = Decimal::from_ratio(
            simulate.actual_minimum_receive,
            Uint128::from(OFFER_AMOUNT_DEFAULT),
        );

        if swap_fee != Decimal::zero() {
            belief_price = belief_price
                .checked_div(Decimal::from_ratio(simulate.swap_ops.len() as u128, 1u128) * swap_fee)
                .unwrap();
        }
        Ok(belief_price)
    }

    pub fn execute_operations(
        &self,
        swap_router: String,
        swap_asset_info: AssetInfo,
        amount: Uint128,
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        swap_to: Option<Addr>,
    ) -> StdResult<CosmosMsg> {
        let cosmos_msg: CosmosMsg = match swap_asset_info {
            AssetInfo::Token { contract_addr } => WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: swap_router,
                    amount,
                    msg: to_binary(&SwapRouterExecuteMsg::ExecuteSwapOperations {
                        operations,
                        minimum_receive,
                        to: swap_to,
                    })?,
                })?,
                funds: vec![],
            }
            .into(),
            AssetInfo::NativeToken { denom } => WasmMsg::Execute {
                contract_addr: swap_router,
                msg: to_binary(&SwapRouterExecuteMsg::ExecuteSwapOperations {
                    operations,
                    minimum_receive,
                    to: swap_to,
                })?,
                funds: vec![coin(amount.u128(), denom)],
            }
            .into(),
        };
        Ok(cosmos_msg)
    }
}
