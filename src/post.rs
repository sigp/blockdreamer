use crate::PostEndpointConfig;
use eth2::types::{BlindedBeaconBlock, EthSpec, Slot};
use itertools::multiunzip;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
pub struct PostEndpoint {
    pub name: String,
    client: Client,
    url: String,
    results_dir: Option<PathBuf>,
    compare_rewards: bool,
    require_all: bool,
    require_same_parent: bool,
    extra_data: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(bound = "E: EthSpec")]
pub struct PostPayload<E: EthSpec> {
    names: Vec<String>,
    labels: Vec<String>,
    blocks: Vec<BlindedBeaconBlock<E>>,
}

impl PostEndpoint {
    pub fn new(config: &PostEndpointConfig) -> Arc<Self> {
        let client = Client::new();
        let name = config.name.clone();
        let url = config.url.clone();
        Arc::new(Self {
            name,
            client,
            url,
            results_dir: config.results_dir.clone(),
            compare_rewards: config.compare_rewards,
            require_all: config.require_all,
            require_same_parent: config.require_same_parent,
            extra_data: config.extra_data,
        })
    }

    pub async fn post_blocks<E: EthSpec>(
        &self,
        names_and_labels: Vec<(String, String)>,
        opt_blocks: Vec<Option<BlindedBeaconBlock<E>>>,
        slot: Slot,
    ) -> Result<(), String> {
        let total_nodes = opt_blocks.len();
        if names_and_labels.len() != opt_blocks.len() {
            return Err(format!(
                "logic error: mismatched blocks and nodes: {} vs {}",
                opt_blocks.len(),
                names_and_labels.len()
            ));
        }

        // Filter out nodes that failed.
        let (names, labels, blocks): (Vec<_>, Vec<_>, Vec<_>) = multiunzip(
            names_and_labels
                .into_iter()
                .zip(opt_blocks)
                .filter_map(|((name, label), opt_block)| Some((name, label, opt_block?))),
        );

        if self.require_all && blocks.len() != total_nodes {
            return Err(format!("only got {}/{} blocks", blocks.len(), total_nodes));
        }

        if self.require_same_parent
            && !blocks
                .iter()
                .all(|block| block.parent_root() == blocks[0].parent_root())
        {
            return Err(format!("not all blocks build on the same parent"));
        }

        let response = if self.extra_data {
            let payload = PostPayload {
                names: names.clone(),
                labels: labels.clone(),
                blocks,
            };

            self.client.post(&self.url).json(&payload)
        } else {
            self.client.post(&self.url).json(&blocks)
        }
        .send()
        .await
        .map_err(|e| format!("POST error: {}", e))?;

        let response_status = response.status();
        let response_text = response
            .text()
            .await
            .unwrap_or_else(|_| "<body garbled>".into());

        if !response_status.is_success() {
            return Err(format!("status {response_status}: {response_text}"));
        }

        let response_json: Vec<Value> = serde_json::from_str(&response_text)
            .map_err(|_| format!("invalid JSON: {response_text}"))?;

        if response_json.len() != names.len() {
            return Err(format!(
                "bad response, only data for {}/{} blocks",
                response_json.len(),
                names.len(),
            ));
        }

        let mut max_reward = 0;
        let mut max_reward_nodes = vec![];

        for ((name, label), result) in names.iter().zip(labels.iter()).zip(response_json) {
            if self.compare_rewards {
                let reward = result["total"].as_u64().unwrap();
                let att_reward = result["attestation_rewards"]["total"].as_u64().unwrap();
                eprintln!(
                    "slot {slot}: calculated rewards for {name}: {reward} gwei (att: {att_reward} gwei)"
                );

                if reward > max_reward {
                    max_reward = reward;
                    max_reward_nodes = vec![name.clone()];
                } else if reward == max_reward {
                    max_reward_nodes.push(name.clone());
                }
            }

            if let Some(results_dir) = &self.results_dir {
                // Store results by client label (same format as blockprint training data).
                let label_dir = results_dir.join(label);
                create_dir_all(&label_dir)
                    .await
                    .map_err(|e| format!("unable to create {}: {}", label_dir.display(), e))?;

                // Name files by node name and slot.
                let result_path = label_dir.join(format!("{name}_{slot}.json"));
                let mut f = File::create(&result_path)
                    .await
                    .map_err(|e| format!("unable to create {}: {}", result_path.display(), e))?;

                let bytes =
                    serde_json::to_vec(&result).map_err(|e| format!("JSON error: {}", e))?;
                f.write_all(&bytes)
                    .await
                    .map_err(|e| format!("unable to write {}: {}", result_path.display(), e))?;
            }
        }

        if self.compare_rewards && names.len() > 1 {
            let max_reward_comma_sep = max_reward_nodes.join(",");
            eprintln!("most profitable block from {max_reward_comma_sep}");
        }

        Ok(())
    }
}
