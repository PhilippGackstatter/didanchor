use std::{collections::HashMap, io::Cursor};

use crypto::hashes::blake2b::Blake2b256;
use futures::StreamExt;
use identity_core::convert::{FromJson, ToJson};
use identity_iota_core::did::IotaDID;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient};
use merkle_tree::Proof;
use packable::{error::UnpackError, unpacker::SliceUnpacker, Packable, PackableExt};

use crate::ChainOfCustody;

/// Storage for Chains of custodies.
#[derive(Clone, Default)]
pub struct ChainStorage {
    ipfs: IpfsClient,
}

impl ChainStorage {
    pub fn new() -> Self {
        Self {
            ipfs: IpfsClient::default(),
        }
    }

    pub async fn add(
        &self,
        verif_chain_of_custody: &VerifiableChainOfCustody,
    ) -> anyhow::Result<String> {
        let coc: Cursor<Vec<u8>> = Cursor::new(verif_chain_of_custody.pack_to_vec());
        let hash = self.ipfs.add(coc).await?.hash;
        Ok(hash)
    }

    // TODO: This needs to be a cluster operation rather than an individual node operation.
    pub async fn unpin(&self, hash: &str) -> anyhow::Result<()> {
        self.ipfs.pin_rm(hash, false).await?;
        Ok(())
    }

    pub async fn get(&self, did: &IotaDID) -> anyhow::Result<Option<VerifiableChainOfCustody>> {
        let index = self.get_index().await?;

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

    pub async fn get_index(&self) -> anyhow::Result<DIDIndex> {
        let cid: String = self.ipfs.name_resolve(None, false, false).await?.path;

        println!("resolved self name to {cid}");

        let json = self.get_bytes(&cid).await?;

        Ok(DIDIndex::from_json_slice(&json)?)
    }

    pub async fn publish_index(&self, index: DIDIndex) -> anyhow::Result<()> {
        let json: Vec<u8> = index.to_json_vec()?;
        let cursor = Cursor::new(json);

        let cid = &self.ipfs.add(cursor).await?.hash;

        println!("published index at {cid}");

        self.ipfs.name_publish(cid, false, None, None, None).await?;

        Ok(())
    }

    async fn get_bytes(&self, cid: &str) -> anyhow::Result<Vec<u8>> {
        let mut stream = self.ipfs.get(cid.as_ref());

        let mut bytes = Vec::new();

        // TODO: What does get even return precisely?
        while let Some(value) = stream.next().await {
            let value = value?;
            bytes.extend_from_slice(&value);
        }

        Ok(bytes)
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

        packer.pack_bytes(bytes.as_slice())?;

        Ok(())
    }

    fn unpack<U: packable::unpacker::Unpacker, const VERIFY: bool>(
        unpacker: &mut U,
    ) -> Result<Self, packable::error::UnpackError<Self::UnpackError, U::Error>> {
        let proof = <Proof<Blake2b256>>::unpack::<_, VERIFY>(unpacker)?;
        let mut bytes = Vec::new();
        unpacker.unpack_bytes(&mut bytes)?;
        let chain_of_custody = ChainOfCustody::from_json_slice(&bytes)
            .map_err(|err| UnpackError::Packable(anyhow::anyhow!(err)))?;

        Ok(Self {
            proof,
            chain_of_custody,
        })
    }
}
