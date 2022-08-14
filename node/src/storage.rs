use std::collections::HashMap;

use crypto::hashes::blake2b::Blake2b256;
use futures::TryStreamExt;
use http::uri::Scheme;
use identity_core::convert::{FromJson, ToJson};
use identity_iota_core::did::IotaDID;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use ipfs_cluster::IpfsClusterClient;
use merkle_tree::Proof;
use packable::{
    error::{UnpackError, UnpackErrorExt},
    unpacker::SliceUnpacker,
    Packable, PackableExt,
};

use crate::ChainOfCustody;

/// Storage for Chains of custodies.
#[derive(Clone, Default)]
pub struct ChainStorage {
    ipfs: IpfsClient,
    ipfs_cluster: IpfsClusterClient,
}

impl ChainStorage {
    pub fn new() -> Self {
        Self {
            ipfs: IpfsClient::default(),
            ipfs_cluster: IpfsClusterClient::default(),
        }
    }

    pub fn new_with_host(
        ipfs_hostname: &str,
        ipfs_port: u16,
        ipfs_cluster_hostname: &str,
    ) -> anyhow::Result<Self> {
        let ipfs = IpfsClient::from_host_and_port(Scheme::HTTP, ipfs_hostname, ipfs_port).unwrap();
        let ipfs_cluster = IpfsClusterClient::new_with_host(ipfs_cluster_hostname);

        Ok(Self { ipfs, ipfs_cluster })
    }

    /// Adds and pins the given [`VerifiableChainOfCustody`].
    pub async fn add(
        &self,
        verif_chain_of_custody: &VerifiableChainOfCustody,
    ) -> anyhow::Result<String> {
        log::debug!(
            "adding coc for {}",
            verif_chain_of_custody.chain_of_custody.0[0].document.id()
        );

        let packed: Vec<u8> = verif_chain_of_custody.pack_to_vec();

        let cid = self.ipfs_cluster.add(packed).await?.cid;

        Ok(cid)
    }

    pub async fn unpin(&self, cid: &str) -> anyhow::Result<()> {
        log::debug!("unpinning {cid}");

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

        let bytes = self.get_bytes(cid).await?;

        let mut unpacker = SliceUnpacker::new(bytes.as_slice());
        let coc: VerifiableChainOfCustody =
            VerifiableChainOfCustody::unpack::<_, false>(&mut unpacker).expect("TODO");

        Ok(Some(coc))
    }

    pub async fn get_index(&self, index_cid: &str) -> anyhow::Result<DIDIndex> {
        log::debug!("retrieving index from {}", index_cid);

        let json = self.get_bytes(index_cid).await?;

        Ok(DIDIndex::from_json_slice(&json)?)
    }

    /// Publishes the given [`DIDIndex`] and returns its CID.
    pub async fn publish_index(&self, index: &DIDIndex) -> anyhow::Result<String> {
        log::debug!("publishing index");
        let json: Vec<u8> = index.to_json_vec()?;

        let cid = self.ipfs_cluster.add(json).await?.cid;

        Ok(cid)
    }

    async fn get_bytes(&self, cid: &str) -> anyhow::Result<Vec<u8>> {
        Ok(self
            .ipfs
            .cat(cid)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await?)
    }
}

/// A map from a DID to the IPFS content id that contains its chain of custody.
pub type DIDIndex = HashMap<IotaDID, String>;

pub struct VerifiableChainOfCustody {
    pub(crate) proof: Proof<Blake2b256>,
    pub(crate) chain_of_custody: ChainOfCustody,
}

impl VerifiableChainOfCustody {
    pub fn new(proof: Proof<Blake2b256>, chain_of_custody: ChainOfCustody) -> Self {
        Self {
            proof,
            chain_of_custody,
        }
    }
}

impl Packable for VerifiableChainOfCustody {
    type UnpackError = anyhow::Error;

    fn pack<P: packable::packer::Packer>(&self, packer: &mut P) -> Result<(), P::Error> {
        self.proof.pack(packer)?;

        let bytes = self
            .chain_of_custody
            .to_json_vec()
            .expect("TODO: unclear how to use P::Error");

        let len: u64 = bytes.len() as u64;

        len.pack(packer)?;
        packer.pack_bytes(bytes.as_slice())?;

        Ok(())
    }

    fn unpack<U: packable::unpacker::Unpacker, const VERIFY: bool>(
        unpacker: &mut U,
    ) -> Result<Self, packable::error::UnpackError<Self::UnpackError, U::Error>> {
        let proof = <Proof<Blake2b256>>::unpack::<_, VERIFY>(unpacker)?;

        let len: usize = u64::unpack::<_, VERIFY>(unpacker).coerce()? as usize;

        let mut bytes = vec![0; len];
        unpacker.unpack_bytes(&mut bytes)?;
        let chain_of_custody = ChainOfCustody::from_json_slice(&bytes)
            .map_err(|err| UnpackError::Packable(anyhow::anyhow!(err)))?;

        Ok(Self {
            proof,
            chain_of_custody,
        })
    }
}
