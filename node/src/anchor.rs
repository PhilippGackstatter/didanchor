use std::collections::HashMap;

use anyhow::Context;
use identity_iota_client::document::ResolvedIotaDocument;
use identity_iota_core::did::IotaDID;
use merkle_tree::Proof;

use crate::{
    AnchorConfig, ChainOfCustody, ChainStorage, DIDIndex, MerkleDIDs, VerifiableChainOfCustody,
};

pub struct Anchor {
    storage: ChainStorage,
    merkle: MerkleDIDs,
    uncommitted_chains: HashMap<IotaDID, ChainOfCustody>,
    index: DIDIndex,
    config: AnchorConfig,
}

impl Anchor {
    pub async fn new() -> anyhow::Result<Self> {
        let storage = ChainStorage::new();

        let config = AnchorConfig::read_default_location().await?;

        let index: DIDIndex = if let Some(ref index_cid) = config.index_cid {
            storage.get_index(index_cid).await?
        } else {
            DIDIndex::new()
        };

        Ok(Self {
            storage,
            merkle: MerkleDIDs::new(),
            uncommitted_chains: HashMap::new(),
            index,
            config,
        })
    }

    pub async fn update_document(&mut self, document: ResolvedIotaDocument) -> anyhow::Result<()> {
        let did = document.document.id().to_owned();

        let chain_of_custody: Option<ChainOfCustody> = match self.uncommitted_chains.remove(&did) {
            coc @ Some(_) => coc,
            None => self
                .storage
                .get(&did, &self.index)
                .await?
                .map(|vcoc| vcoc.chain_of_custody),
        };

        let chain_of_custody: ChainOfCustody =
            self.merkle.update_document(chain_of_custody, document)?;

        self.uncommitted_chains.insert(did, chain_of_custody);

        Ok(())
    }

    pub async fn commit_changes(&mut self) -> anyhow::Result<()> {
        let mut uncommitted_chains = HashMap::new();

        std::mem::swap(&mut self.uncommitted_chains, &mut uncommitted_chains);

        for (did, coc) in uncommitted_chains.into_iter() {
            let proof: Proof<_> = self
                .merkle
                .generate_merkle_proof(&did)
                .context("should be contained in the tree")?;

            // Store the proof together with the COC in storage.
            let vcoc = VerifiableChainOfCustody::new(proof, coc);
            let content_id: String = self.storage.add(&vcoc).await?;

            if let Some(cid) = self.index.get(&did) {
                // Remove the previous pin as we no longer need it.
                // In a production deployment, this would probably have to be done later
                // to ensure availability within a certain grace period.
                self.storage.unpin(cid).await?;
            }

            // Update the storage index.
            self.index.insert(did, content_id);
        }

        let index_cid = self.storage.publish_index(&self.index).await?;
        self.config.index_cid = Some(index_cid);

        self.config.write_default_location().await?;

        // TODO: Store merkle root in alias output.

        Ok(())
    }
}
