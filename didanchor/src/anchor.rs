use did_common::{ChainOfCustody, VerifiableChainOfCustody};
use identity_core::convert::ToJson;
use std::{collections::HashMap, time::Instant};

use anyhow::Context;
use identity_iota_client::document::ResolvedIotaDocument;
use identity_iota_core::did::IotaDID;
use iota_client::block::output::AliasId;
use merkle_tree::Proof;

use crate::{
    resolve_alias_content, AliasContent, AnchorConfig, AnchorOutput, ChainStorage, DIDIndex,
    IpfsNodeAddress, MerkleDIDs,
};

pub struct Anchor {
    storage: ChainStorage,
    merkle: MerkleDIDs,
    uncommitted_chains: HashMap<IotaDID, ChainOfCustody>,
    index: DIDIndex,
    config: AnchorConfig,
    anchor_output: AnchorOutput,
    index_cid: Option<String>,
}

impl Anchor {
    pub async fn new() -> anyhow::Result<Self> {
        let config: AnchorConfig = AnchorConfig::read_default_location().await?;

        let anchor_output: AnchorOutput = AnchorOutput::new(
            config.mnemonic.clone(),
            config.alias_id,
            &config.iota_endpoint,
        )?;

        // Retrieve the current alias output to obtain the latest index cid.
        // We could store this locally, but this way seems safer overall.
        let content: Option<AliasContent> =
            resolve_alias_content(&anchor_output.client, config.alias_id).await?;

        let storage = ChainStorage::new(
            config.ipfs_cluster_addrs.clone(),
            config.ipfs_node_addrs.clone(),
        )?;

        let (index, index_cid): (DIDIndex, Option<String>) = if let Some(content) = content {
            (
                storage.get_index(&content.index_cid).await?,
                Some(content.index_cid),
            )
        } else {
            (DIDIndex::new(), None)
        };

        Ok(Self {
            storage,
            merkle: MerkleDIDs::new(),
            uncommitted_chains: HashMap::new(),
            index,
            config,
            anchor_output,
            index_cid,
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

    pub async fn commit_changes(&mut self) -> anyhow::Result<AliasId> {
        let time = Instant::now();
        let changes_to_commit = self.uncommitted_chains.len();

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

        // Unpin old index and upload and set new one.
        if let Some(ref old_index_cid) = self.index_cid {
            self.storage.unpin(old_index_cid).await?;
        }
        let index_cid: String = self.storage.publish_index(&self.index).await?;
        self.index_cid = Some(index_cid.clone());

        // Update the Alias Output.

        let content = AliasContent::new(
            index_cid,
            // self.config.ipfs_gateway_addrs.clone(),
            // TODO:
            vec![IpfsNodeAddress {
                hostname: "http://127.0.0.1".to_owned(),
                swarm_port: 4001,
                gateway_port: 0,
                cluster_port: 0,
            }],
            self.merkle.merkle_root(),
        );

        let alias_id = self.anchor_output.publish_output(content).await?;

        self.config.alias_id = alias_id;

        self.config.write_default_location().await?;

        log::debug!(
            "committed {changes_to_commit} change(s) in {}s",
            time.elapsed().as_secs()
        );

        Ok(alias_id)
    }
}
