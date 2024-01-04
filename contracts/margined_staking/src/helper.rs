use cosmwasm_std::{Addr, Response, StdResult, Uint128};
use margined_common::asset::AssetInfo;
use margined_utils::contracts::helpers::FeePoolController;

pub fn create_distribute_message_and_update_response(
    mut response: Response,
    fee_pool: Addr,
    asset_info: AssetInfo,
    amount: Uint128,
    recipient: String,
) -> StdResult<Response> {
    let token = match asset_info {
        AssetInfo::NativeToken { denom } => denom,
        AssetInfo::Token { contract_addr } => contract_addr.to_string(),
    };

    if !amount.is_zero() {
        let fee_pool_controller = FeePoolController(fee_pool);
        let distribute_msg = fee_pool_controller.send_token(token, amount, recipient)?;

        response = response.add_message(distribute_msg);
    };

    Ok(response)
}
