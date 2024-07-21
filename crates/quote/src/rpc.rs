use crate::{
    error::FailedQuote,
    quote::{QuoteResult, QuoteTarget},
};
use alloy_ethers_typecast::{
    multicall::{
        IMulticall3::{aggregate3Call, Call3},
        MULTICALL3_ADDRESS,
    },
    transaction::{ReadContractParameters, ReadableClient, ReadableClientError},
};
use alloy_primitives::{hex::FromHex, Address, U64};
use alloy_sol_types::SolCall;
use rain_error_decoding::AbiDecodedErrorType;
use rain_orderbook_bindings::IOrderBookV4::quoteCall;

/// Quotes array of given orders using the given rpc url
pub async fn multi_quote(
    quote_targets: &[QuoteTarget],
    rpc: &str,
    block_number: Option<u64>,
    multicall_address: Option<Address>,
) -> Result<Vec<QuoteResult>, ReadableClientError> {
    let client = ReadableClient::new_from_url(rpc.to_string())?;
    let parameters = ReadContractParameters {
        address: multicall_address.unwrap_or(Address::from_hex(MULTICALL3_ADDRESS).unwrap()),
        block_number: block_number.map(U64::from),
        call: aggregate3Call {
            calls: quote_targets
                .iter()
                .map(|quote_target| Call3 {
                    allowFailure: true,
                    target: quote_target.orderbook,
                    callData: quoteCall {
                        quoteConfig: quote_target.quote.clone(),
                    }
                    .abi_encode(),
                })
                .collect(),
        },
    };
    let multicall_result = client.read(parameters).await?;

    let mut result: Vec<QuoteResult> = vec![];
    for res in multicall_result.returnData {
        if res.success {
            match quoteCall::abi_decode_returns(&res.returnData, true) {
                Ok(v) => {
                    if v.exists {
                        result.push(Ok(v.into()));
                    } else {
                        result.push(Err(FailedQuote::NonExistent));
                    }
                }
                Err(e) => result.push(Err(FailedQuote::CorruptReturnData(e.to_string()))),
            }
        } else {
            match AbiDecodedErrorType::selector_registry_abi_decode(&res.returnData).await {
                Ok(e) => result.push(Err(FailedQuote::RevertError(e))),
                Err(e) => result.push(Err(FailedQuote::RevertErrorDecodeFailed(e))),
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quote::OrderQuote;
    use alloy_ethers_typecast::multicall::IMulticall3::Result as MulticallResult;
    use alloy_ethers_typecast::{
        request_shim::{AlloyTransactionRequest, TransactionRequestShim},
        rpc::{eip2718::TypedTransaction, BlockNumber, Request, Response},
    };
    use alloy_primitives::{hex::encode_prefixed, U256};
    use alloy_sol_types::SolValue;
    use httpmock::{Method::POST, MockServer};
    use rain_orderbook_bindings::IOrderBookV4::Quote;
    use serde_json::{from_str, Value};

    #[tokio::test]
    async fn test_multi_quote() {
        let rpc_server = MockServer::start_async().await;

        let orderbook = Address::random();
        let multicall = Address::from_hex(MULTICALL3_ADDRESS).unwrap();

        // build call data
        let quote_targets = vec![
            QuoteTarget {
                quote: Quote {
                    ..Default::default()
                },
                orderbook,
            },
            QuoteTarget {
                quote: Quote {
                    ..Default::default()
                },
                orderbook,
            },
            QuoteTarget {
                quote: Quote {
                    ..Default::default()
                },
                orderbook,
            },
        ];
        let call = aggregate3Call {
            calls: quote_targets
                .iter()
                .map(|quote_target| Call3 {
                    allowFailure: true,
                    target: quote_target.orderbook,
                    callData: quoteCall {
                        quoteConfig: quote_target.quote.clone(),
                    }
                    .abi_encode(),
                })
                .collect(),
        };

        // build response data
        let response_data = vec![
            MulticallResult {
                success: true,
                returnData: quoteCall::abi_encode_returns(&(true, U256::from(1), U256::from(2))),
            },
            MulticallResult {
                success: true,
                returnData: quoteCall::abi_encode_returns(&(false, U256::ZERO, U256::ZERO)),
            },
            MulticallResult {
                success: false,
                returnData: vec![],
            },
        ]
        .abi_encode();

        // mock rpc with call data and response data
        rpc_server.mock(|when, then| {
            when.method(POST).path("/").json_body_partial(
                Request::<(TypedTransaction, BlockNumber)>::eth_call_request(
                    1,
                    TypedTransaction::Eip1559(
                        AlloyTransactionRequest::new()
                            .with_to(Some(multicall))
                            .with_data(Some(call.abi_encode()))
                            .to_eip1559(),
                    ),
                    None,
                )
                .to_json_string()
                .unwrap(),
            );
            then.json_body_obj(
                &from_str::<Value>(
                    &Response::new_success(1, encode_prefixed(response_data).as_str())
                        .to_json_string()
                        .unwrap(),
                )
                .unwrap(),
            );
        });

        let result = multi_quote(&quote_targets, rpc_server.url("/").as_str(), None, None)
            .await
            .unwrap();
        let mut iter_result = result.into_iter();

        assert_eq!(
            iter_result.next().unwrap().unwrap(),
            OrderQuote {
                max_output: U256::from(1),
                ratio: U256::from(2),
            }
        );
        matches!(
            iter_result.next().unwrap(),
            Result::Err(FailedQuote::NonExistent)
        );
        matches!(
            iter_result.next().unwrap(),
            Result::Err(FailedQuote::RevertErrorDecodeFailed(_))
        );
        assert!(iter_result.next().is_none());
    }
}
