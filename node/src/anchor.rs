use std::collections::HashMap;

use anyhow::Context;
use identity_iota_client::document::ResolvedIotaDocument;
use identity_iota_core::did::IotaDID;
use merkle_tree::Proof;

use crate::{ChainOfCustody, ChainStorage, DIDIndex, MerkleDIDs, VerifiableChainOfCustody};

pub struct Anchor {
    storage: ChainStorage,
    merkle: MerkleDIDs,
    uncommitted_chains: HashMap<IotaDID, ChainOfCustody>,
}

impl Anchor {
    pub fn new(hostname: &str, port: u16) -> anyhow::Result<Self> {
        let storage = ChainStorage::new_with_host(hostname, port)?;

        Ok(Self {
            storage,
            merkle: MerkleDIDs::new(),
            uncommitted_chains: HashMap::new(),
        })
    }

    pub async fn update_document(&mut self, document: ResolvedIotaDocument) -> anyhow::Result<()> {
        let did = document.document.id().to_owned();
        // TODO: Check uncommitted_chains first, only then go to storage.
        let chain_of_custody: Option<ChainOfCustody> = self
            .storage
            .get(&did)
            .await?
            .map(|vcoc| vcoc.chain_of_custody);
        let chain_of_custody: ChainOfCustody =
            self.merkle.update_document(chain_of_custody, document)?;

        self.uncommitted_chains.insert(did, chain_of_custody);

        Ok(())
    }

    pub async fn commit_changes(&mut self) -> anyhow::Result<()> {
        let mut uncommitted_chains = HashMap::new();

        std::mem::swap(&mut self.uncommitted_chains, &mut uncommitted_chains);

        let mut index: DIDIndex = self.storage.get_index().await?.unwrap_or_default();

        for (did, coc) in uncommitted_chains.into_iter() {
            let proof: Proof<_> = self
                .merkle
                .generate_merkle_proof(&did)
                .context("should be contained in the tree")?;

            // Store the proof together with the COC in storage.
            let vcoc = VerifiableChainOfCustody::new(proof, coc);
            let content_id: String = self.storage.add(&vcoc).await?;

            // Update the storage index.
            index.insert(did, content_id);
        }

        self.storage.publish_index(index).await?;

        // TODO: Store merkle root in alias output.

        Ok(())
    }
}
