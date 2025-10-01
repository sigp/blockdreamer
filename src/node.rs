use crate::config::Node as NodeConfig;
use eth2::{
    types::{
        BlindedBeaconBlock, ChainSpec, EthSpec, FullBlockContents, ProduceBlockV3Metadata,
        ProduceBlockV3Response, Signature, SignatureBytes, SkipRandaoVerification, Slot,
    },
    BeaconNodeHttpClient, Timeouts,
};
use sensitive_url::SensitiveUrl;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct Node {
    pub config: Arc<NodeConfig>,
    pub client: BeaconNodeHttpClient,
    pub spec: Arc<ChainSpec>,
}

impl Node {
    pub fn new(config: Arc<NodeConfig>, spec: Arc<ChainSpec>) -> Result<Self, String> {
        let url = SensitiveUrl::parse(&config.url).map_err(|e| format!("Invalid URL: {:?}", e))?;
        let client = BeaconNodeHttpClient::new(url, Timeouts::set_all(Duration::from_secs(6)));
        Ok(Self {
            config,
            client,
            spec,
        })
    }

    pub async fn get_block_v3_json<E: EthSpec>(
        &self,
        slot: Slot,
        randao_reveal: &SignatureBytes,
        skip_randao_verification: SkipRandaoVerification,
        builder_boost_factor: Option<u64>,
    ) -> Result<(BlindedBeaconBlock<E>, ProduceBlockV3Metadata), String> {
        let (response, metadata) = self
            .client
            .get_validator_blocks_v3_modular::<E>(
                slot,
                randao_reveal,
                None,
                skip_randao_verification,
                builder_boost_factor,
            )
            .await
            .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))?;

        match response.data {
            ProduceBlockV3Response::Full(block_contents) => {
                // Throw away the blobs for now.
                Ok((block_contents.block().to_ref().into(), metadata))
            }
            ProduceBlockV3Response::Blinded(block) => Ok((block, metadata)),
        }
    }

    pub async fn get_block_v3_ssz<E: EthSpec>(
        &self,
        slot: Slot,
        randao_reveal: &SignatureBytes,
        skip_randao_verification: SkipRandaoVerification,
        builder_boost_factor: Option<u64>,
    ) -> Result<(BlindedBeaconBlock<E>, ProduceBlockV3Metadata), String> {
        let (response, metadata) = self
            .client
            .get_validator_blocks_v3_modular_ssz::<E>(
                slot,
                randao_reveal,
                None,
                skip_randao_verification,
                builder_boost_factor,
            )
            .await
            .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))?;

        match response {
            ProduceBlockV3Response::Full(block_contents) => {
                // Throw away the blobs for now.
                Ok((block_contents.block().to_ref().into(), metadata))
            }
            ProduceBlockV3Response::Blinded(block) => Ok((block, metadata)),
        }
    }

    pub async fn get_block<E: EthSpec>(
        &self,
        slot: Slot,
        builder_boost_factor: Option<u64>,
    ) -> Result<(BlindedBeaconBlock<E>, Option<ProduceBlockV3Metadata>), String> {
        let randao_reveal = Signature::infinity().unwrap().into();
        let skip_randao_verification = if self.config.skip_randao_verification {
            SkipRandaoVerification::Yes
        } else {
            SkipRandaoVerification::No
        };
        if self.config.v3 {
            if self.config.ssz {
                self.get_block_v3_ssz(
                    slot,
                    &randao_reveal,
                    skip_randao_verification,
                    builder_boost_factor,
                )
                .await
            } else {
                self.get_block_v3_json(
                    slot,
                    &randao_reveal,
                    skip_randao_verification,
                    builder_boost_factor,
                )
                .await
            }
            .map(|(block, metadata)| (block, Some(metadata)))
        } else if self.config.ssz {
            self.get_block_v2_ssz(slot, &randao_reveal, skip_randao_verification)
                .await
        } else {
            self.get_block_v2_json(slot, &randao_reveal, skip_randao_verification)
                .await
        }
    }

    pub async fn get_block_v2_json<E: EthSpec>(
        &self,
        slot: Slot,
        randao_reveal: &SignatureBytes,
        skip_randao_verification: SkipRandaoVerification,
    ) -> Result<(BlindedBeaconBlock<E>, Option<ProduceBlockV3Metadata>), String> {
        let block_contents = self
            .client
            .get_validator_blocks_modular::<E>(slot, randao_reveal, None, skip_randao_verification)
            .await
            .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))?;
        Ok((block_contents.data().block().to_ref().into(), None))
    }

    pub async fn get_block_v2_ssz<E: EthSpec>(
        &self,
        slot: Slot,
        randao_reveal: &SignatureBytes,
        skip_randao_verification: SkipRandaoVerification,
    ) -> Result<(BlindedBeaconBlock<E>, Option<ProduceBlockV3Metadata>), String> {
        let bytes = self
            .client
            .get_validator_blocks_modular_ssz::<E>(
                slot,
                randao_reveal,
                None,
                skip_randao_verification,
            )
            .await
            .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))?
            .ok_or_else(|| {
                format!(
                    "Error fetching block from {}: returned 404",
                    self.config.url
                )
            })?;
        let block_contents = FullBlockContents::from_ssz_bytes(&bytes, &self.spec)
            .map_err(|e| format!("Error fetching block from {}: {e:?}", self.config.url))?;
        Ok((block_contents.block().to_ref().into(), None))
    }

    pub async fn get_block_with_timeout<E: EthSpec>(
        &self,
        slot: Slot,
        builder_boost_factor: Option<u64>,
    ) -> Result<(BlindedBeaconBlock<E>, Option<ProduceBlockV3Metadata>), String> {
        tokio::time::timeout(
            Duration::from_secs(6),
            self.get_block(slot, builder_boost_factor),
        )
        .await
        .map_err(|_| format!("request to {} timed out after 6s", self.config.name))?
    }
}
