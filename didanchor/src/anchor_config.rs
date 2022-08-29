use std::path::Path;

use identity_core::convert::{FromJson, ToJson};
use iota_client::block::output::AliasId;
use url::Url;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnchorConfig {
    pub alias_id: AliasId,
    pub mnemonic: String,
    pub iota_endpoint: String,
    pub ipfs_node_addrs: Vec<Url>,
    pub ipfs_cluster_addrs: Vec<Url>,
}

impl AnchorConfig {
    pub const DEFAULT_PATH: &'static str = "./anchor_config.json";

    pub async fn read(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        log::debug!("reading config from {}", path.as_ref().display());

        match tokio::fs::read(path).await {
            Ok(content) => Ok(AnchorConfig::from_json(std::str::from_utf8(
                content.as_slice(),
            )?)?),
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }

    pub async fn write(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        log::debug!("writing config to {}", path.as_ref().display());
        tokio::fs::write(path, self.to_json_pretty()?).await?;
        Ok(())
    }

    pub async fn read_default_location() -> anyhow::Result<Self> {
        Self::read(Self::DEFAULT_PATH).await
    }

    pub async fn write_default_location(&self) -> anyhow::Result<()> {
        self.write(Self::DEFAULT_PATH).await
    }
}
