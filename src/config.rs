use serde::Deserialize;
use std::path::PathBuf;
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    sync::Arc,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub network: Option<String>,
    pub network_dir: Option<PathBuf>,
    pub canonical_bn: String,
    /// URLs to push the dreamt blocks to (probably blockgauge).
    #[serde(default)]
    pub post_endpoints: Vec<PostEndpointConfig>,
    pub nodes: Vec<Arc<Node>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub name: String,
    pub label: String,
    pub url: String,
    #[serde(default)]
    pub skip_randao_verification: bool,
    // Deprecated.
    #[serde(default)]
    pub use_builder: bool,
    #[serde(default = "default_true")]
    pub ssz: bool,
    #[serde(default)]
    pub v3: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub builder_boost_factor: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostEndpointConfig {
    pub name: String,
    /// URL to send data to. HTTPS and basic auth are both supported.
    pub url: String,
    /// Directory to save post responses to.
    pub results_dir: Option<PathBuf>,
    /// Whether to post extra data about the nodes that produced the blocks. Default: true.
    #[serde(default = "default_true")]
    pub extra_data: bool,
    /// Whether to compare attestation rewards after POSTing to the endpoint. Default: false.
    ///
    /// Assumes the `post_endpoint` is blockgauge or Lighthouse's `block_rewards` endpoint.
    #[serde(default)]
    pub compare_rewards: bool,
    /// Only post blocks if all clients return a block. Default: false.
    #[serde(default)]
    pub require_all: bool,
    /// Only post blocks if all blocks have the same parent. Default: false.
    #[serde(default)]
    pub require_same_parent: bool,
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, io::Error> {
        let mut f = File::open(path)?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        Ok(toml::from_str(&s).unwrap())
    }
}

fn default_true() -> bool {
    true
}
