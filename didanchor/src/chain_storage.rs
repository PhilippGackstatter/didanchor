use std::collections::HashMap;

use bytes::Bytes;
use did_common::VerifiableChainOfCustody;
use identity_core::convert::{FromJson, ToJson};
use identity_iota_core::did::IotaDID;
use ipfs_client::IpfsClient;
use ipfs_cluster::IpfsCluster;
use packable::{unpacker::SliceUnpacker, Packable, PackableExt};
use url::Url;

/// Storage for Chains of custodies.
#[derive(Clone)]
pub struct ChainStorage {
    ipfs_client: IpfsClient,
    ipfs_cluster: IpfsCluster,
}

impl ChainStorage {
    pub fn new(ipfs_cluster_addrs: Vec<Url>, ipfs_node_addrs: Vec<Url>) -> anyhow::Result<Self> {
        Ok(Self {
            ipfs_client: IpfsClient::new(ipfs_node_addrs)?,
            ipfs_cluster: IpfsCluster::new(ipfs_cluster_addrs)?,
        })
    }

    /// Adds and pins the given [`VerifiableChainOfCustody`].
    pub async fn add(
        &self,
        verif_chain_of_custody: &VerifiableChainOfCustody,
    ) -> anyhow::Result<String> {
        log::debug!(
            "ipfs add {}",
            verif_chain_of_custody.chain_of_custody.0[0].document.id()
        );

        let packed: Vec<u8> = verif_chain_of_custody.pack_to_vec();

        let cid = self.ipfs_cluster.add(packed).await?.cid;

        Ok(cid)
    }

    pub async fn unpin(&self, cid: &str) -> anyhow::Result<()> {
        log::debug!("ipfs pin rm {cid}");

        self.ipfs_cluster.unpin(cid).await?;

        Ok(())
    }

    pub async fn get(
        &self,
        did: &IotaDID,
        index: &DIDIndex,
    ) -> anyhow::Result<Option<VerifiableChainOfCustody>> {
        let cid = if let Some(cid) = index.get(did) {
            cid
        } else {
            return Ok(None);
        };

        let bytes: Bytes = self.get_bytes(cid).await?;

        let mut unpacker = SliceUnpacker::new(bytes.as_ref());
        let coc: VerifiableChainOfCustody =
            VerifiableChainOfCustody::unpack::<_, false>(&mut unpacker).expect("TODO");

        Ok(Some(coc))
    }

    pub async fn get_index(&self, index_cid: &str) -> anyhow::Result<DIDIndex> {
        log::debug!("retrieving index from {}", index_cid);

        let json: Bytes = self.get_bytes(index_cid).await?;

        Ok(DIDIndex::from_json_slice(&json)?)
    }

    /// Publishes the given [`DIDIndex`] and returns its CID.
    pub async fn publish_index(&self, index: &DIDIndex) -> anyhow::Result<String> {
        log::debug!("publishing index");
        let json: Vec<u8> = index.to_json_vec()?;

        let cid = self.ipfs_cluster.add(json).await?.cid;

        Ok(cid)
    }

    async fn get_bytes(&self, cid: &str) -> anyhow::Result<Bytes> {
        self.ipfs_client.cat(cid).await
    }
}

/// A map from a DID to the IPFS content id that contains its chain of custody.
pub type DIDIndex = HashMap<IotaDID, String>;
