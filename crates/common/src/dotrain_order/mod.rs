use crate::{
    add_order::{ORDERBOOK_ADDORDER_POST_TASK_ENTRYPOINTS, ORDERBOOK_ORDER_ENTRYPOINTS},
    rainlang::compose_to_rainlang,
};
use alloy::primitives::Address;
use alloy_ethers_typecast::transaction::{ReadableClient, ReadableClientError};
use dotrain::{error::ComposeError, RainDocument};
use futures::future::join_all;
use rain_interpreter_parser::{ParserError, ParserV2};
pub use rain_metadata::types::authoring::v2::*;
use rain_orderbook_app_settings::{
    config_source::{ConfigSource, ConfigSourceError},
    merge::MergeError,
    Config, ParseConfigSourceError,
};
use rain_orderbook_env::GH_COMMIT_SHA;
use serde::Serialize;
use thiserror::Error;
use typeshare::typeshare;

pub mod filter;

#[derive(Debug, Clone, PartialEq)]
pub struct DotrainOrder {
    config: Config,
    dotrain: String,
    config_source: ConfigSource,
}

#[derive(Error, Debug)]
pub enum DotrainOrderError {
    #[error(transparent)]
    ConfigSourceError(#[from] ConfigSourceError),

    #[error(transparent)]
    ParseConfigSourceError(#[from] ParseConfigSourceError),

    #[error("Scenario {0} not found")]
    ScenarioNotFound(String),

    #[error("Metaboard {0} not found")]
    MetaboardNotFound(String),

    #[error(transparent)]
    ComposeError(#[from] ComposeError),

    #[error(transparent)]
    MergeConfigError(#[from] MergeError),

    #[error(transparent)]
    AuthoringMetaV2Error(#[from] AuthoringMetaV2Error),

    #[error(transparent)]
    FetchAuthoringMetaV2WordError(#[from] FetchAuthoringMetaV2WordError),

    #[error(transparent)]
    ReadableClientError(#[from] ReadableClientError),

    #[error(transparent)]
    ParserError(#[from] ParserError),

    #[error("{0}")]
    CleanUnusedFrontmatterError(String),

    #[error("Raindex version mismatch: got {1}, should be {0}")]
    RaindexVersionMismatch(String, String),

    #[error("Raindex version missing: should be {0}")]
    MissingRaindexVersion(String),
}

#[typeshare]
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum WordsResult {
    Success(AuthoringMetaV2),
    Error(String),
}

#[typeshare]
#[derive(Serialize, Debug, Clone)]
pub struct ContractWords {
    pub address: Address,
    pub words: WordsResult,
}

impl From<Result<AuthoringMetaV2, DotrainOrderError>> for WordsResult {
    fn from(result: Result<AuthoringMetaV2, DotrainOrderError>) -> Self {
        match result {
            Ok(meta) => WordsResult::Success(meta),
            Err(err) => WordsResult::Error(err.to_string()),
        }
    }
}

#[typeshare]
#[derive(Serialize, Debug, Clone)]
pub struct ScenarioWords {
    pub scenario: String,
    pub pragma_words: Vec<ContractWords>,
    pub deployer_words: ContractWords,
}

impl DotrainOrder {
    pub async fn new(dotrain: String, config: Option<String>) -> Result<Self, DotrainOrderError> {
        match config {
            Some(config) => {
                let config_string = ConfigSource::try_from_string(config.clone()).await?;
                let frontmatter = RainDocument::get_front_matter(&dotrain).unwrap();
                let mut frontmatter_config =
                    ConfigSource::try_from_string(frontmatter.to_string()).await?;
                frontmatter_config.merge(config_string)?;
                Ok(Self {
                    dotrain,
                    config_source: frontmatter_config.clone(),
                    config: frontmatter_config.try_into()?,
                })
            }
            None => {
                let frontmatter = RainDocument::get_front_matter(&dotrain).unwrap();
                let config_source = ConfigSource::try_from_string(frontmatter.to_string()).await?;
                Ok(Self {
                    dotrain,
                    config_source: config_source.clone(),
                    config: config_source.try_into()?,
                })
            }
        }
    }

    // get this instance's config
    pub fn config(&self) -> &Config {
        &self.config
    }

    // get this instance's config source
    pub fn config_source(&self) -> &ConfigSource {
        &self.config_source
    }

    // get this instance's dotrain string
    pub fn dotrain(&self) -> &str {
        &self.dotrain
    }

    // get this instance's config mut
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    // get this instance's config source mut
    pub fn config_source_mut(&mut self) -> &mut ConfigSource {
        &mut self.config_source
    }

    pub async fn compose_scenario_to_rainlang(
        &self,
        scenario: String,
    ) -> Result<String, DotrainOrderError> {
        let scenario = self
            .config
            .scenarios
            .get(&scenario)
            .ok_or_else(|| DotrainOrderError::ScenarioNotFound(scenario))?;

        Ok(compose_to_rainlang(
            self.dotrain.clone(),
            scenario.bindings.clone(),
            &ORDERBOOK_ORDER_ENTRYPOINTS,
        )?)
    }

    pub async fn compose_scenario_to_post_task_rainlang(
        &self,
        scenario: String,
    ) -> Result<String, DotrainOrderError> {
        let scenario = self
            .config
            .scenarios
            .get(&scenario)
            .ok_or_else(|| DotrainOrderError::ScenarioNotFound(scenario))?;

        Ok(compose_to_rainlang(
            self.dotrain.clone(),
            scenario.bindings.clone(),
            &ORDERBOOK_ADDORDER_POST_TASK_ENTRYPOINTS,
        )?)
    }

    pub async fn get_pragmas_for_scenario(
        &self,
        scenario: &str,
    ) -> Result<Vec<Address>, DotrainOrderError> {
        let deployer = &self
            .config
            .scenarios
            .get(scenario)
            .ok_or_else(|| DotrainOrderError::ScenarioNotFound(scenario.to_string()))?
            .deployer;
        let parser: ParserV2 = deployer.address.into();
        let rainlang = self
            .compose_scenario_to_rainlang(scenario.to_string())
            .await?;

        let client = ReadableClient::new_from_url(deployer.network.rpc.clone().to_string())?;
        let pragmas = parser.parse_pragma_text(&rainlang, client).await?;
        Ok(pragmas)
    }

    pub async fn get_contract_authoring_meta_v2_for_scenario(
        &self,
        scenario: &str,
        address: Address,
    ) -> Result<AuthoringMetaV2, DotrainOrderError> {
        let network = &self
            .config
            .scenarios
            .get(scenario)
            .ok_or_else(|| DotrainOrderError::ScenarioNotFound(scenario.to_string()))?
            .deployer
            .network;

        let rpc = &network.rpc;
        let metaboard = self
            .config
            .metaboards
            .get(&network.name)
            .ok_or_else(|| DotrainOrderError::MetaboardNotFound(network.name.clone()))?
            .clone();
        Ok(
            AuthoringMetaV2::fetch_for_contract(address, rpc.to_string(), metaboard.to_string())
                .await?,
        )
    }

    pub async fn get_deployer_words_for_scenario(
        &self,
        scenario: &str,
    ) -> Result<ContractWords, DotrainOrderError> {
        let deployer = &self
            .config
            .scenarios
            .get(scenario)
            .ok_or_else(|| DotrainOrderError::ScenarioNotFound(scenario.to_string()))?
            .deployer
            .address;

        Ok(ContractWords {
            address: *deployer,
            words: self
                .get_contract_authoring_meta_v2_for_scenario(scenario, *deployer)
                .await
                .into(),
        })
    }

    pub async fn get_pragma_words_for_scenario(
        &self,
        scenario: &str,
    ) -> Result<Vec<ContractWords>, DotrainOrderError> {
        let pragma_addresses = self.get_pragmas_for_scenario(scenario).await?;
        let mut futures = vec![];

        for pragma in &pragma_addresses {
            futures.push(self.get_contract_authoring_meta_v2_for_scenario(scenario, *pragma));
        }

        Ok(pragma_addresses
            .into_iter()
            .zip(join_all(futures).await)
            .map(|(address, words)| ContractWords {
                address,
                words: words.into(),
            })
            .collect())
    }

    pub async fn get_all_words_for_scenario(
        &self,
        scenario: &str,
    ) -> Result<ScenarioWords, DotrainOrderError> {
        let deployer = &self
            .config
            .scenarios
            .get(scenario)
            .ok_or_else(|| DotrainOrderError::ScenarioNotFound(scenario.to_string()))?
            .deployer
            .address;
        let mut addresses = vec![*deployer];
        addresses.extend(self.get_pragmas_for_scenario(scenario).await?);

        let mut futures = vec![];
        for address in addresses.clone() {
            futures.push(self.get_contract_authoring_meta_v2_for_scenario(scenario, address));
        }
        let mut results = join_all(futures).await;

        let deployer_words = ContractWords {
            address: *deployer,
            words: results.drain(0..1).nth(0).unwrap().into(),
        };
        let pragma_words = results
            .into_iter()
            .enumerate()
            .map(|(i, v)| ContractWords {
                address: addresses[i + 1],
                words: v.into(),
            })
            .collect();

        Ok(ScenarioWords {
            scenario: scenario.to_string(),
            pragma_words,
            deployer_words,
        })
    }

    pub async fn get_all_scenarios_all_words(
        &self,
    ) -> Result<Vec<ScenarioWords>, DotrainOrderError> {
        let mut scenarios = vec![];
        for scenario in self.config.scenarios.keys() {
            scenarios.push(self.get_all_words_for_scenario(scenario).await?);
        }
        Ok(scenarios)
    }

    pub async fn validate_raindex_version(&self) -> Result<(), DotrainOrderError> {
        let app_sha = GH_COMMIT_SHA.to_string();

        if let Some(raindex_version) = &self.config.raindex_version {
            if app_sha != *raindex_version {
                return Err(DotrainOrderError::RaindexVersionMismatch(
                    app_sha,
                    raindex_version.to_string(),
                ));
            }
        } else {
            return Err(DotrainOrderError::MissingRaindexVersion(app_sha));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::{hex::encode_prefixed, primitives::B256, sol, sol_types::SolValue};
    use alloy_ethers_typecast::rpc::Response;
    use httpmock::MockServer;
    use rain_metadata::{KnownMagic, RainMetaDocumentV1Item};
    use serde_bytes::ByteBuf;

    sol!(
        struct AuthoringMetaV2Sol {
            bytes32 word;
            string description;
        }
    );
    sol!(
        struct PragmaV1 { address[] usingWordsFrom; }
    );

    #[tokio::test]
    async fn test_config_parse() {
        let server = mock_server(vec![]);
        let dotrain = format!(
            r#"
networks:
    polygon:
        rpc: {rpc_url}
        chain-id: 137
        network-id: 137
        currency: MATIC
deployers:
    polygon:
        address: 0x1234567890123456789012345678901234567890
scenarios:
    polygon:
---
#calculate-io
_ _: 0 0;
#handle-io
:;"#,
            rpc_url = server.url("/rpc"),
        );

        let dotrain_order = DotrainOrder::new(dotrain.to_string(), None).await.unwrap();

        assert_eq!(
            dotrain_order
                .config
                .networks
                .get("polygon")
                .unwrap()
                .rpc
                .to_string(),
            server.url("/rpc"),
        );
    }

    #[tokio::test]
    async fn test_rainlang_from_scenario() {
        let server = mock_server(vec![]);
        let dotrain = format!(
            r#"
networks:
    polygon:
        rpc: {rpc_url}
        chain-id: 137
        network-id: 137
        currency: MATIC
deployers:
    polygon:
        address: 0x1234567890123456789012345678901234567890
scenarios:
    polygon:
---
#calculate-io
_ _: 0 0;
#handle-io
:;"#,
            rpc_url = server.url("/rpc"),
        );

        let dotrain_order = DotrainOrder::new(dotrain.to_string(), None).await.unwrap();

        let rainlang = dotrain_order
            .compose_scenario_to_rainlang("polygon".to_string())
            .await
            .unwrap();

        assert_eq!(
            rainlang,
            r#"/* 0. calculate-io */ 
_ _: 0 0;

/* 1. handle-io */ 
:;"#
        );
    }

    #[tokio::test]
    async fn test_rainlang_post_from_scenario() {
        let server = mock_server(vec![]);
        let dotrain = format!(
            r#"
networks:
    polygon:
        rpc: {rpc_url}
        chain-id: 137
        network-id: 137
        currency: MATIC
deployers:
    polygon:
        address: 0x1234567890123456789012345678901234567890
scenarios:
    polygon:
---
#calculate-io
_ _: 0 0;
#handle-io
:;
#handle-add-order
_ _: 1 2;
"#,
            rpc_url = server.url("/rpc"),
        );

        let dotrain_order = DotrainOrder::new(dotrain.to_string(), None).await.unwrap();

        let rainlang = dotrain_order
            .compose_scenario_to_post_task_rainlang("polygon".to_string())
            .await
            .unwrap();

        assert_eq!(
            rainlang,
            r#"/* 0. handle-add-order */ 
_ _: 1 2;"#
        );
    }

    #[tokio::test]
    async fn test_config_merge() {
        let server = mock_server(vec![]);
        let dotrain = format!(
            r#"
networks:
  polygon:
    rpc: {rpc_url}
    chain-id: 137
    network-id: 137
    currency: MATIC
---
#calculate-io
_ _: 00;

#handle-io
:;"#,
            rpc_url = server.url("/rpc-polygon"),
        );

        let settings = format!(
            r#"
networks:
    mainnet:
        rpc: {rpc_url}
        chain-id: 1
        network-id: 1
        currency: ETH"#,
            rpc_url = server.url("/rpc-mainnet"),
        );

        let merged_dotrain_order =
            DotrainOrder::new(dotrain.to_string(), Some(settings.to_string()))
                .await
                .unwrap();

        assert_eq!(
            merged_dotrain_order
                .config
                .networks
                .get("mainnet")
                .unwrap()
                .rpc
                .to_string(),
            server.url("/rpc-mainnet")
        );
    }

    #[tokio::test]
    async fn test_get_pragmas_for_scenario() {
        let pragma_addresses = vec![Address::random()];
        let server = mock_server(pragma_addresses.clone());
        let dotrain = format!(
            r#"
networks:
    sepolia:
        rpc: {rpc_url}
        chain-id: 0
deployers:
    sepolia:
        address: 0x017F5651eB8fa4048BBc17433149c6c035d391A6
scenarios:
    sepolia:
---
#calculate-io
using-words-from 0xb06202aA3Fe7d85171fB7aA5f17011d17E63f382
_: order-hash(),
_ _: 0 0;
#handle-io
:;"#,
            rpc_url = server.url("/rpc"),
        );

        let dotrain_order = DotrainOrder::new(dotrain.to_string(), None).await.unwrap();

        let pragmas = dotrain_order
            .get_pragmas_for_scenario("sepolia")
            .await
            .unwrap();

        assert_eq!(pragmas, pragma_addresses);
    }

    #[tokio::test]
    async fn test_get_contract_authoring_meta_v2_for_scenario() {
        let pragma_addresses = vec![Address::random()];
        let server = mock_server(pragma_addresses.clone());
        let dotrain = format!(
            r#"
    networks:
        sepolia:
            rpc: {rpc_url}
            chain-id: 0
    deployers:
        sepolia:
            address: 0x3131baC3E2Ec97b0ee93C74B16180b1e93FABd59
    scenarios:
        sepolia:
    metaboards:
        sepolia: {metaboard_url}
    ---
    #calculate-io
    using-words-from 0xbc609623F5020f6Fc7481024862cD5EE3FFf52D7
    _: order-hash(),
    _ _: 0 0;
    #handle-io
    :;"#,
            rpc_url = server.url("/rpc"),
            metaboard_url = server.url("/sg"),
        );

        let dotrain_order = DotrainOrder::new(dotrain.to_string(), None).await.unwrap();

        let result = dotrain_order
            .get_contract_authoring_meta_v2_for_scenario("sepolia", pragma_addresses[0])
            .await
            .unwrap();

        assert_eq!(&result.words[0].word, "some-word");
        assert_eq!(&result.words[0].description, "some-desc");

        assert_eq!(&result.words[1].word, "some-other-word");
        assert_eq!(&result.words[1].description, "some-other-desc");
    }

    #[tokio::test]
    async fn test_get_pragma_words_for_scenario() {
        let pragma_addresses = vec![Address::random()];
        let server = mock_server(pragma_addresses.clone());
        let dotrain = format!(
            r#"
    networks:
        sepolia:
            rpc: {rpc_url}
            chain-id: 0
    deployers:
        sepolia:
            address: 0x3131baC3E2Ec97b0ee93C74B16180b1e93FABd59
    scenarios:
        sepolia:
    metaboards:
        sepolia: {metaboard_url}
    ---
    #calculate-io
    using-words-from 0xbc609623F5020f6Fc7481024862cD5EE3FFf52D7
    _: order-hash(),
    _ _: 0 0;
    #handle-io
    :;"#,
            rpc_url = server.url("/rpc"),
            metaboard_url = server.url("/sg"),
        );

        let dotrain_order = DotrainOrder::new(dotrain.to_string(), None).await.unwrap();
        let result = dotrain_order
            .get_pragma_words_for_scenario("sepolia")
            .await
            .unwrap();

        assert!(result.len() == 1);
        assert_eq!(result[0].address, pragma_addresses[0]);
        assert!(matches!(result[0].words, WordsResult::Success(_)));
        if let WordsResult::Success(authoring_meta) = &result[0].words {
            assert_eq!(&authoring_meta.words[0].word, "some-word");
            assert_eq!(&authoring_meta.words[0].description, "some-desc");

            assert_eq!(&authoring_meta.words[1].word, "some-other-word");
            assert_eq!(&authoring_meta.words[1].description, "some-other-desc");
        }
    }

    #[tokio::test]
    async fn test_get_deployer_words_for_scenario() {
        let server = mock_server(vec![]);
        let deployer = Address::random();
        let dotrain = format!(
            r#"
    networks:
        sepolia:
            rpc: {rpc_url}
            chain-id: 0
    deployers:
        sepolia:
            address: {deployer_address}
    scenarios:
        sepolia:
    metaboards:
        sepolia: {metaboard_url}
    ---
    #calculate-io
    using-words-from 0xbc609623F5020f6Fc7481024862cD5EE3FFf52D7
    _: order-hash(),
    _ _: 0 0;
    #handle-io
    :;"#,
            rpc_url = server.url("/rpc"),
            metaboard_url = server.url("/sg"),
            deployer_address = encode_prefixed(deployer),
        );

        let dotrain_order = DotrainOrder::new(dotrain.to_string(), None).await.unwrap();
        let result = dotrain_order
            .get_deployer_words_for_scenario("sepolia")
            .await
            .unwrap();

        assert_eq!(result.address, deployer);
        assert!(matches!(result.words, WordsResult::Success(_)));
        if let WordsResult::Success(authoring_meta) = &result.words {
            assert_eq!(&authoring_meta.words[0].word, "some-word");
            assert_eq!(&authoring_meta.words[0].description, "some-desc");

            assert_eq!(&authoring_meta.words[1].word, "some-other-word");
            assert_eq!(&authoring_meta.words[1].description, "some-other-desc");
        }
    }

    #[tokio::test]
    async fn test_get_all_words_for_scenario() {
        let deployer = Address::random();
        let pragma_addresses = vec![Address::random()];
        let server = mock_server(pragma_addresses.clone());
        let dotrain = format!(
            r#"
    networks:
        sepolia:
            rpc: {rpc_url}
            chain-id: 0
    deployers:
        sepolia:
            address: {deployer_address}
    scenarios:
        sepolia:
    metaboards:
        sepolia: {metaboard_url}
    ---
    #calculate-io
    using-words-from 0xbc609623F5020f6Fc7481024862cD5EE3FFf52D7
    _: order-hash(),
    _ _: 0 0;
    #handle-io
    :;"#,
            rpc_url = server.url("/rpc"),
            metaboard_url = server.url("/sg"),
            deployer_address = encode_prefixed(deployer),
        );

        let dotrain_order = DotrainOrder::new(dotrain.to_string(), None).await.unwrap();
        let result = dotrain_order
            .get_all_words_for_scenario("sepolia")
            .await
            .unwrap();

        assert_eq!(&result.scenario, "sepolia");

        assert_eq!(result.deployer_words.address, deployer);
        assert!(matches!(
            result.deployer_words.words,
            WordsResult::Success(_)
        ));
        if let WordsResult::Success(authoring_meta) = &result.deployer_words.words {
            assert_eq!(&authoring_meta.words[0].word, "some-word");
            assert_eq!(&authoring_meta.words[0].description, "some-desc");

            assert_eq!(&authoring_meta.words[1].word, "some-other-word");
            assert_eq!(&authoring_meta.words[1].description, "some-other-desc");
        }

        assert!(result.pragma_words.len() == 1);
        assert_eq!(result.pragma_words[0].address, pragma_addresses[0]);
        assert!(matches!(
            result.pragma_words[0].words,
            WordsResult::Success(_)
        ));
        if let WordsResult::Success(authoring_meta) = &result.pragma_words[0].words {
            assert_eq!(&authoring_meta.words[0].word, "some-word");
            assert_eq!(&authoring_meta.words[0].description, "some-desc");

            assert_eq!(&authoring_meta.words[1].word, "some-other-word");
            assert_eq!(&authoring_meta.words[1].description, "some-other-desc");
        }
    }

    // helper function to mock rpc and sg response
    fn mock_server(with_pragma_addresses: Vec<Address>) -> MockServer {
        let server = MockServer::start();
        // mock contract calls
        server.mock(|when, then| {
            when.path("/rpc").body_contains("0x01ffc9a7ffffffff");
            then.body(
                Response::new_success(1, &B256::left_padding_from(&[0]).to_string())
                    .to_json_string()
                    .unwrap(),
            );
        });
        server.mock(|when, then| {
            when.path("/rpc").body_contains("0x01ffc9a7");
            then.body(
                Response::new_success(1, &B256::left_padding_from(&[1]).to_string())
                    .to_json_string()
                    .unwrap(),
            );
        });
        server.mock(|when, then| {
            when.path("/rpc").body_contains("0x6f5aa28d");
            then.body(
                Response::new_success(1, &B256::random().to_string())
                    .to_json_string()
                    .unwrap(),
            );
        });
        server.mock(|when, then| {
            when.path("/rpc").body_contains("0x5514ca20");
            then.body(
                Response::new_success(
                    1,
                    &encode_prefixed(
                        PragmaV1 {
                            usingWordsFrom: with_pragma_addresses,
                        }
                        .abi_encode(),
                    ),
                )
                .to_json_string()
                .unwrap(),
            );
        });

        // mock sg query
        server.mock(|when, then| {
            when.path("/sg");
            then.status(200).json_body_obj(&serde_json::json!({
                "data": {
                    "metaV1S": [{
                        "meta": encode_prefixed(
                            RainMetaDocumentV1Item {
                                payload: ByteBuf::from(
                                    vec![
                                        AuthoringMetaV2Sol {
                                            word: B256::right_padding_from("some-word".as_bytes()),
                                            description: "some-desc".to_string(),
                                        },
                                        AuthoringMetaV2Sol {
                                            word: B256::right_padding_from("some-other-word".as_bytes()),
                                            description: "some-other-desc".to_string(),
                                        }
                                    ]
                                    .abi_encode(),
                                ),
                                magic: KnownMagic::AuthoringMetaV2,
                                content_type: rain_metadata::ContentType::OctetStream,
                                content_encoding: rain_metadata::ContentEncoding::None,
                                content_language: rain_metadata::ContentLanguage::None,
                            }
                            .cbor_encode()
                            .unwrap()
                        ),
                        "metaHash": "0x00",
                        "sender": "0x00",
                        "id": "0x00",
                        "metaBoard": {
                            "id": "0x00",
                            "metas": [],
                            "address": "0x00",
                        },
                        "subject": "0x00",
                    }]
                }
            }));
        });
        server
    }

    #[tokio::test]
    async fn test_validate_raindex_version_happy() {
        let dotrain = format!(
            r#"
                raindex-version: {GH_COMMIT_SHA}
                networks:
                    sepolia:
                        rpc: http://example.com
                        chain-id: 0
                deployers:
                    sepolia:
                        address: 0x3131baC3E2Ec97b0ee93C74B16180b1e93FABd59
                ---
                #calculate-io
                _ _: 0 0;
                #handle-io
                :;"#,
            GH_COMMIT_SHA = GH_COMMIT_SHA,
        );

        let dotrain_order = DotrainOrder::new(dotrain.to_string(), None).await.unwrap();

        dotrain_order.validate_raindex_version().await.unwrap();
    }

    #[tokio::test]
    async fn test_validate_raindex_version_unhappy() {
        let dotrain = format!(
            r#"
                raindex-version: {GH_COMMIT_SHA}
                networks:
                    sepolia:
                        rpc: http://example.com
                        chain-id: 0
                deployers:
                    sepolia:
                        address: 0x3131baC3E2Ec97b0ee93C74B16180b1e93FABd59
                ---
                #calculate-io
                _ _: 0 0;
                #handle-io
                :;"#,
            GH_COMMIT_SHA = "1234567890",
        );

        let dotrain_order = DotrainOrder::new(dotrain.to_string(), None).await.unwrap();

        assert!(dotrain_order.validate_raindex_version().await.is_err());
    }
}