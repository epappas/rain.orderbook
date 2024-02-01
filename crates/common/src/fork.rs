use super::error::ForkCallError;
use super::error::{abi_decode_error, AbiDecodedErrorType};
use forker::*;
use once_cell::sync::Lazy;
use revm::primitives::Bytes;
use std::{collections::HashMap, sync::Mutex};

/// static hashmap of fork evm instances, used for caching instances between runs
pub static FORKS: Lazy<Mutex<HashMap<String, ForkedEvm>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn fork_call<'a>(
    fork_url: &str,
    fork_block_number: u64,
    from_address: &[u8],
    to_address: &[u8],
    calldata: &[u8],
) -> Result<Result<Bytes, AbiDecodedErrorType>, ForkCallError<'a>> {
    // build key from fork url and block number
    let key = fork_url.to_owned() + &fork_block_number.to_string();

    let is_cached = { FORKS.lock()?.contains_key(&key) };

    let result = if is_cached {
        let mut forks = FORKS.lock()?;
        let forked_evm = forks
            .get_mut(&key)
            .ok_or("could not get the cached forked evm".to_owned())?;

        // call a contract read-only
        forked_evm
            .call(from_address, to_address, calldata)
            .map_err(|e| e.to_string())?
    } else {
        let mut forked_evm =
            ForkedEvm::new(fork_url, Some(fork_block_number), Some(200000u64), None).await;

        // call a contract read-only
        let res = forked_evm
            .call(from_address, to_address, calldata)
            .map_err(|e| e.to_string())?;

        // lock static FORKS
        let mut forks = FORKS.lock()?;
        forks.insert(key, forked_evm);
        res
    };

    if result.reverted {
        // decode result bytes to error selectors if it was a revert
        Ok(Err(abi_decode_error(&result.result).await?))
    } else {
        Ok(Ok(result.result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex::decode;
    use alloy_sol_types::SolValue;

    const FORK_URL: &str = "https://rpc.ankr.com/polygon_mumbai";
    const FORK_BLOCK_NUMBER: u64 = 45122616;
    const DEPLOYER_ADDRESS: &str = "0x5155cE66E704c5Ce79a0c6a1b79113a6033a999b";
    const PARSER_ADDRESS: &str = "0xea3b12393D2EFc4F3E15D41b30b3d020610B9e02";
    const FROM_ADDRESS: &str = "0x5855A7b48a1f9811392B89F18A8e27347EF84E42";
    const DEPLOY_EXPRESSION_2_SELECTOR: &str = "0xb7f14403"; // deployExpression2(bytes,uint256[])
    const PARSE_SELECTOR: &str = "0xfab4087a"; // parse()

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_fork_call_parse_fail_parse() {
        // has no semi at the end
        let rainlang_text = r"_: int-add(1)";
        let mut calldata = decode(PARSE_SELECTOR).unwrap();
        calldata.extend_from_slice(&rainlang_text.abi_encode()); // extend with rainlang text to build calldata

        // this is calling parse() that will not run integrity checks
        // in order to run integrity checks another call should be done on
        // expressionDeployer2() of deployer contract with same process
        let result = fork_call(
            FORK_URL,
            FORK_BLOCK_NUMBER,
            &decode(FROM_ADDRESS).unwrap(),
            &decode(PARSER_ADDRESS).unwrap(),
            &calldata,
        )
        .await
        .expect("test failed!");
        let expected = Err(AbiDecodedErrorType::Known {
            name: "MissingFinalSemi".to_owned(),
            args: vec!["Uint(0x000000000000000000000000000000000000000000000000000000000000000d_U256, 256)".to_owned()],
            sig: "MissingFinalSemi(uint256)".to_owned(),
        });
        assert_eq!(result, expected);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_fork_call_parse_fail_integrity() {
        // fixed semi error, but still has bad input problem
        // get expressionconfig and call deployer to get integrity checks error
        let rainlang_text = r"_: int-add(1);";
        let mut calldata = decode(PARSE_SELECTOR).unwrap();
        calldata.extend_from_slice(&rainlang_text.abi_encode()); // extend with rainlang text
        let expression_config = fork_call(
            FORK_URL,
            FORK_BLOCK_NUMBER,
            &decode(FROM_ADDRESS).unwrap(),
            &decode(PARSER_ADDRESS).unwrap(),
            &calldata,
        )
        .await
        .unwrap()
        .unwrap();

        let mut calldata = decode(DEPLOY_EXPRESSION_2_SELECTOR).unwrap();
        calldata.extend_from_slice(&expression_config); // extend with result of parse() which is expressionConfig

        // get integrity check results, if ends with error, decode with the selectors
        let result = fork_call(
            FORK_URL,
            FORK_BLOCK_NUMBER,
            &decode(FROM_ADDRESS).unwrap(),
            &decode(DEPLOYER_ADDRESS).unwrap(),
            &calldata,
        )
        .await
        .expect("test failed!");
        let expected = Err(AbiDecodedErrorType::Known {
            name: "BadOpInputsLength".to_owned(),
            args: vec![
                "Uint(0x0000000000000000000000000000000000000000000000000000000000000001_U256, 256)".to_owned(), 
                "Uint(0x0000000000000000000000000000000000000000000000000000000000000002_U256, 256)".to_owned(), 
                "Uint(0x0000000000000000000000000000000000000000000000000000000000000001_U256, 256)".to_owned()
            ],
            sig: "BadOpInputsLength(uint256,uint256,uint256)".to_owned(),
        });
        assert_eq!(result, expected);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_fork_call_parse_success() {
        // get expressionconfig and call deployer to get integrity checks error
        let rainlang_text = r"_: int-add(1 2);";
        let mut calldata = decode(PARSE_SELECTOR).unwrap();
        calldata.extend_from_slice(&rainlang_text.abi_encode()); // extend with rainlang text
        let expression_config = fork_call(
            FORK_URL,
            FORK_BLOCK_NUMBER,
            &decode(FROM_ADDRESS).unwrap(),
            &decode(PARSER_ADDRESS).unwrap(),
            &calldata,
        )
        .await
        .unwrap()
        .unwrap();

        let mut calldata = decode(DEPLOY_EXPRESSION_2_SELECTOR).unwrap();
        calldata.extend_from_slice(&expression_config); // extend with result of parse() which is expressionConfig

        // expression deploys ok so the expressionConfig in previous step can be used to deploy onchain
        let result = fork_call(
            FORK_URL,
            FORK_BLOCK_NUMBER,
            &decode(FROM_ADDRESS).unwrap(),
            &decode(DEPLOYER_ADDRESS).unwrap(),
            &calldata,
        )
        .await
        .expect("test failed");
        let expected = Ok(
            Bytes::from(
                alloy_primitives::hex::decode(
                    "0x000000000000000000000000f22cda7695125487993110d706f3b001c8d106400000000000000000000000008a326d777bc34ea563bd21854b436d458112185b00000000000000000000000064c9b10e815a089698521b10be95c8c9c2ed0b3c000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000020001000000000000000000000000000000000000000000000000000000000000").unwrap()
                )
            );
        assert_eq!(result, expected);
    }
}