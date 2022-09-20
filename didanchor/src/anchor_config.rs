use std::path::Path;

use iota_client::block::output::AliasId;

use crate::{IpfsNodeManagementAddress, IpfsNodePublicAddress};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnchorConfig {
    pub alias_id: AliasId,
    pub mnemonic: String,
    pub iota_endpoint: String,
    pub ipfs_node_public_addrs: Vec<IpfsNodePublicAddress>,
    pub ipfs_node_management_addrs: Vec<IpfsNodeManagementAddress>,
}

impl AnchorConfig {
    pub const DEFAULT_PATH: &'static str = "./anchor_config.toml";

    pub async fn read(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        log::debug!("reading config from {}", path.as_ref().display());

        match tokio::fs::read(path).await {
            Ok(content) => Ok(toml::from_slice::<AnchorConfig>(content.as_slice())?),
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }

    pub async fn write(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        log::debug!("writing config to {}", path.as_ref().display());
        tokio::fs::write(path, toml::to_string_pretty(&self)?).await?;
        Ok(())
    }

    pub async fn read_default_location() -> anyhow::Result<Self> {
        Self::read(Self::DEFAULT_PATH).await
    }

    pub async fn write_default_location(&self) -> anyhow::Result<()> {
        self.write(Self::DEFAULT_PATH).await
    }
}
