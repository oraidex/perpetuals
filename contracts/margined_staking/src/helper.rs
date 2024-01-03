use cosmwasm_std::{to_binary, CosmosMsg, Response, StdResult, Uint128, WasmMsg};
use margined_common::asset::AssetInfo;
use margined_perp::margined_fee_pool::ExecuteMsg as FeeExecuteMsg;

pub fn create_distribute_message_and_update_response(
    mut response: Response,
    fee_collector: String,
    asset_info: AssetInfo,
    amount: Uint128,
    recipient: String,
) -> StdResult<Response> {
    let token = match asset_info {
        AssetInfo::NativeToken { denom } => denom,
        AssetInfo::Token { contract_addr } => contract_addr.to_string(),
    };

    if !amount.is_zero() {
        let distribute_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: fee_collector,
            msg: to_binary(&FeeExecuteMsg::SendToken {
                token,
                amount,
                recipient,
            })
            .unwrap(),
            funds: vec![],
        });

        response = response.add_message(distribute_msg);
    };

    Ok(response)
}
