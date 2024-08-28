use crate::execute::Execute;
use anyhow::{anyhow, Result};
use clap::{ArgAction, Args, Parser};
use csv::Writer;
use rain_orderbook_common::dotrain_order::{AuthoringMetaV2, DotrainOrder, DotrainOrderError};
use reqwest::Url;
use std::{fs::read_to_string, path::PathBuf, str::FromStr, sync::Arc};

/// Get words of a deployer contract from the given inputs
#[derive(Debug, Parser)]
pub struct Words {
    #[command(flatten)]
    pub input: Input,

    #[command(flatten)]
    pub source: Source,

    /// Only get pragma words for a given scenario
    #[arg(
        long,
        requires = "scenario",
        action = ArgAction::SetTrue,
        conflicts_with_all = ["deployer_only", "deployer"],
    )]
    pub pragma_only: bool,

    /// Only get deployer words for a given scenario
    #[arg(
        long,
        requires = "scenario",
        action = ArgAction::SetTrue,
        conflicts_with_all = ["pragma_only", "deployer"],
    )]
    pub deployer_only: bool,

    /// Optional metaboard subgraph url, will override the metaboard in
    /// inputs or if inputs has no metaboard specified inside
    #[arg(short = 'm', long, value_name = "URL")]
    pub metaboard_subgraph: Option<String>,

    /// Optional output file path to write the result into
    #[arg(short = 'o', long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Print the result on console (send result to std out)
    #[arg(long, action = ArgAction::SetTrue)]
    pub stdout: bool,
}

/// Group of possible input files, at least one of dotrain file or
/// setting yml file or both
#[derive(Args, Clone, Debug, PartialEq)]
#[group(required = true, multiple = true)]
pub struct Input {
    /// Path to the .rain file specifying the order
    #[arg(short = 'f', long, value_name = "PATH")]
    pub dotrain_file: Option<PathBuf>,

    /// Path to the settings yaml file
    #[arg(short = 'c', long, value_name = "PATH")]
    pub settings_file: Option<PathBuf>,
}

/// Group of possible sources, only one of deployer or scenario
#[derive(Args, Clone, Debug, PartialEq)]
#[group(required = true, multiple = false)]
pub struct Source {
    /// Deployer key to get its associating words
    #[arg(short = 'd', long)]
    pub deployer: Option<String>,

    /// Scenario key, requires dotrain_file if used
    #[arg(short = 's', long, requires = "dotrain_file")]
    pub scenario: Option<String>,
}

impl Execute for Words {
    async fn execute(&self) -> Result<()> {
        let dotrain = self
            .input
            .dotrain_file
            .as_ref()
            .and_then(|v| read_to_string(v).ok())
            .unwrap_or("---\n".to_string());
        let settings = match &self.input.settings_file {
            Some(settings_file) => {
                Some(read_to_string(settings_file.clone()).map_err(|e| anyhow!(e))?)
            }
            None => None,
        };
        let mut order = DotrainOrder::new(dotrain, settings).await?;

        let results = if let Some(deployer_key) = &self.source.deployer {
            // get deployer from order config
            let deployer = order
                .config
                .deployers
                .get(deployer_key)
                .ok_or(anyhow!("undefined deployer!"))?;

            // get metaboard subgraph url
            let metaboard_url = self
                .metaboard_subgraph
                .as_ref()
                .map(|v| v.to_string())
                .or_else(|| {
                    order
                        .config
                        .metaboards
                        .get(&deployer.network.name)
                        .map(|v| v.to_string())
                })
                .ok_or(anyhow!("undefined metaboard subgraph url"))?;

            AuthoringMetaV2::fetch_for_contract(
                deployer.address,
                deployer.network.rpc.to_string(),
                metaboard_url,
            )
            .await?
            .words
        } else if let Some(scenario) = &self.source.scenario {
            // set the cli given metaboard url into the config
            if let Some(v) = &self.metaboard_subgraph {
                let network_name = &order
                    .config
                    .scenarios
                    .get(scenario)
                    .ok_or(anyhow!("undefined scenario"))?
                    .deployer
                    .network
                    .name;
                order
                    .config
                    .metaboards
                    .insert(network_name.to_string(), Arc::new(Url::from_str(v)?));
            }
            if self.deployer_only {
                order.get_scenario_deployer_words(scenario).await?.words
            } else if self.pragma_only {
                order
                    .get_scenario_pragma_words(scenario)
                    .await?
                    .1
                    .into_iter()
                    .collect::<Result<Vec<AuthoringMetaV2>, DotrainOrderError>>()?
                    .into_iter()
                    .flat_map(|v| v.words)
                    .collect()
            } else {
                order
                    .get_scenario_all_words(scenario)
                    .await?
                    .1
                    .into_iter()
                    .collect::<Result<Vec<AuthoringMetaV2>, DotrainOrderError>>()?
                    .into_iter()
                    .flat_map(|v| v.words)
                    .collect()
            }
        } else {
            // clap doesnt allow this to happen since at least 1 source
            // is required which is enforced and catched by clap
            panic!("undefined source")
        };

        let mut csv_writer = Writer::from_writer(vec![]);
        for item in results.clone().into_iter() {
            csv_writer.serialize(item)?;
        }
        let text = String::from_utf8(csv_writer.into_inner()?)?;

        if let Some(output) = &self.output {
            std::fs::write(output, &text)?;
        }
        if self.stdout {
            println!("{}", text);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::{hex::encode_prefixed, primitives::B256, sol, sol_types::SolValue};
    use alloy_ethers_typecast::rpc::Response;
    use clap::CommandFactory;
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

    #[test]
    fn verify_command1() {
        Words::command().debug_assert();
    }

    #[tokio::test]
    async fn test_execute_happy_with_dotrain() {
        let server = mock_server();
        let dotrain_content = format!(
            "
networks:
    some-network:
        rpc: {}
        chain-id: 123
        network-id: 123
        currency: ETH

metaboards:
    some-network: {}

deployers:
    some-deployer:
        network: some-network
        address: 0xF14E09601A47552De6aBd3A0B165607FaFd2B5Ba
---
#binding
:;",
            server.url("/rpc"),
            server.url("/sg")
        );
        let dotrain_path = "./test_dotrain_words_happy.rain";
        std::fs::write(dotrain_path, dotrain_content).unwrap();

        let words = Words {
            input: Input {
                dotrain_file: Some(dotrain_path.into()),
                settings_file: None,
            },
            source: Source {
                deployer: Some("some-deployer".to_string()),
                scenario: None,
            },
            pragma_only: false,
            deployer_only: false,
            metaboard_subgraph: None,
            output: None,
            stdout: true,
        };

        // should execute successfully
        assert!(words.execute().await.is_ok());

        // remove test file
        std::fs::remove_file(dotrain_path).unwrap();
    }

    #[tokio::test]
    async fn test_execute_happy_with_settings() {
        let server = mock_server();
        let settings_content = format!(
            "
networks:
    some-network:
        rpc: {}
        chain-id: 123
        network-id: 123
        currency: ETH

metaboards:
    some-network: {}

deployers:
    some-deployer:
        network: some-network
        address: 0xF14E09601A47552De6aBd3A0B165607FaFd2B5Ba",
            server.url("/rpc"),
            server.url("/sg")
        );
        let settings_path = "./test_settings_words_happy.yml";
        std::fs::write(settings_path, settings_content).unwrap();

        let words = Words {
            input: Input {
                settings_file: Some(settings_path.into()),
                dotrain_file: None,
            },
            source: Source {
                deployer: Some("some-deployer".to_string()),
                scenario: None,
            },
            pragma_only: false,
            deployer_only: false,
            metaboard_subgraph: None,
            output: None,
            stdout: true,
        };

        // should execute successfully
        assert!(words.execute().await.is_ok());

        // remove test file
        std::fs::remove_file(settings_path).unwrap();
    }

    #[tokio::test]
    async fn test_execute_happy_all() {
        let server = mock_server();
        let dotrain_content = format!(
            "
metaboards:
    some-network: {}
---
#binding\n:;",
            server.url("/sg")
        );
        let settings_content = format!(
            "
networks:
    some-network:
        rpc: {}
        chain-id: 123
        network-id: 123
        currency: ETH

deployers:
    some-deployer:
        network: some-network
        address: 0xF14E09601A47552De6aBd3A0B165607FaFd2B5Ba",
            server.url("/rpc"),
        );
        let settings_path = "./test_settings_words_happy_all.yml";
        std::fs::write(settings_path, settings_content).unwrap();
        let dotrain_path = "./test_dotrain_words_happy_all.rain";
        std::fs::write(dotrain_path, dotrain_content).unwrap();

        let words = Words {
            input: Input {
                settings_file: Some(settings_path.into()),
                dotrain_file: Some(dotrain_path.into()),
            },
            source: Source {
                deployer: Some("some-deployer".to_string()),
                scenario: None,
            },
            pragma_only: false,
            deployer_only: false,
            metaboard_subgraph: None,
            output: None,
            stdout: true,
        };

        // should execute successfully
        assert!(words.execute().await.is_ok());

        // remove test files
        std::fs::remove_file(settings_path).unwrap();
        std::fs::remove_file(dotrain_path).unwrap();
    }

    #[tokio::test]
    async fn test_execute_happy_scenario_all_words() {
        let server = mock_server();
        let dotrain_content = "---\n#calculate-io\n_ _: 1 2;\n#handle-io\n:;".to_string();
        let settings_content = format!(
            "
networks:
    some-network:
        rpc: {}
        chain-id: 123
        network-id: 123
        currency: ETH

deployers:
    some-deployer:
        network: some-network
        address: 0xF14E09601A47552De6aBd3A0B165607FaFd2B5Ba

scenarios:
    some-scenario:
        network: some-network
        deployer: some-deployer
",
            server.url("/rpc"),
        );
        let settings_path = "./test_settings_all_words_happy_all.yml";
        std::fs::write(settings_path, settings_content).unwrap();
        let dotrain_path = "./test_dotrain_all_words_happy_all.rain";
        std::fs::write(dotrain_path, dotrain_content).unwrap();

        let words = Words {
            input: Input {
                settings_file: Some(settings_path.into()),
                dotrain_file: Some(dotrain_path.into()),
            },
            source: Source {
                deployer: None,
                scenario: Some("some-scenario".to_string()),
            },
            pragma_only: false,
            deployer_only: false,
            metaboard_subgraph: Some(server.url("/sg").to_string()),
            output: None,
            stdout: true,
        };

        // should execute successfully
        assert!(words.execute().await.is_ok());

        // remove test files
        std::fs::remove_file(settings_path).unwrap();
        std::fs::remove_file(dotrain_path).unwrap();
    }

    #[tokio::test]
    async fn test_execute_happy_scenario_deployer_words() {
        let server = mock_server();
        let dotrain_content = format!(
            "
metaboards:
    some-network: {}
---
#binding\n:;",
            server.url("/sg")
        );
        let settings_content = format!(
            "
networks:
    some-network:
        rpc: {}
        chain-id: 123
        network-id: 123
        currency: ETH

deployers:
    some-deployer:
        network: some-network
        address: 0xF14E09601A47552De6aBd3A0B165607FaFd2B5Ba

scenarios:
    some-scenario:
        network: some-network
        deployer: some-deployer
",
            server.url("/rpc"),
        );
        let settings_path = "./test_settings_deployer_words_happy_all.yml";
        std::fs::write(settings_path, settings_content).unwrap();
        let dotrain_path = "./test_dotrain_deployer_words_happy_all.rain";
        std::fs::write(dotrain_path, dotrain_content).unwrap();

        let words = Words {
            input: Input {
                settings_file: Some(settings_path.into()),
                dotrain_file: Some(dotrain_path.into()),
            },
            source: Source {
                deployer: None,
                scenario: Some("some-scenario".to_string()),
            },
            pragma_only: false,
            deployer_only: true,
            metaboard_subgraph: None,
            output: None,
            stdout: true,
        };

        // should execute successfully
        assert!(words.execute().await.is_ok());

        // remove test files
        std::fs::remove_file(settings_path).unwrap();
        std::fs::remove_file(dotrain_path).unwrap();
    }

    #[tokio::test]
    async fn test_execute_unhappy() {
        let server = MockServer::start();
        // mock contract calls that doesnt implement IDescribeByMetaV1
        server.mock(|when, then| {
            when.path("/rpc").body_contains("0x01ffc9a701ffc9a7");
            then.body(
                Response::new_success(1, &B256::left_padding_from(&[0]).to_string())
                    .to_json_string()
                    .unwrap(),
            );
        });

        let dotrain_content = format!(
            "
networks:
    some-network:
        rpc: {}
        chain-id: 123
        network-id: 123
        currency: ETH

metaboards:
    some-network: {}

deployers:
    some-deployer:
        network: some-network
        address: 0xF14E09601A47552De6aBd3A0B165607FaFd2B5Ba
---
#binding
:;",
            server.url("/rpc"),
            server.url("/sg")
        );
        let dotrain_path = "./test_dotrain_words_unhappy.rain";
        std::fs::write(dotrain_path, dotrain_content).unwrap();

        let words = Words {
            input: Input {
                dotrain_file: Some(dotrain_path.into()),
                settings_file: None,
            },
            source: Source {
                deployer: Some("some-deployer".to_string()),
                scenario: None,
            },
            pragma_only: false,
            deployer_only: false,
            metaboard_subgraph: None,
            output: None,
            stdout: true,
        };

        // should fail
        assert!(words.execute().await.is_err());

        // remove test file
        std::fs::remove_file(dotrain_path).unwrap();
    }

    // helper function to mock rpc and sg response
    fn mock_server() -> MockServer {
        let server = MockServer::start();
        // mock contract calls
        server.mock(|when, then| {
            when.path("/rpc").body_contains("0x01ffc9a701ffc9a7");
            then.body(
                Response::new_success(1, &B256::left_padding_from(&[1]).to_string())
                    .to_json_string()
                    .unwrap(),
            );
        });
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
                            usingWordsFrom: vec![],
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
            when.path("/sg"); // You need to tailor this to the actual body sent
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
}
