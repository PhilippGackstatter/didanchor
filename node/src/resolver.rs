use identity_core::convert::FromJson;
use identity_did::{
    did::{CoreDID, DID},
    document::CoreDocument,
};
use identity_iota_core::did::IotaDID;
use iota_client::{
    api_types::responses::OutputResponse,
    block::output::{AliasId, AliasOutput, Output, OutputId},
    Client,
};

use crate::{tangle, AliasContent, ChainStorage, DIDIndex};

pub struct Resolver {
    storage: ChainStorage,
    client: Client,
}

impl Resolver {
    pub fn new(hostname: &str, port: u16) -> anyhow::Result<Self> {
        let storage = ChainStorage::new_with_host(hostname, port, "http://127.0.0.1:9094")?;

        let client: Client = Client::builder()
            .with_primary_node(tangle::IOTA_NETWORK_ENDPOINT, None)?
            .finish()?;

        Ok(Self { storage, client })
    }

    /// Resolve the given did into its corresponding DID document.
    ///
    /// Ensures validity in the chain of custody, as well as ensuring it is the version of the CoC
    /// committed to by the anchoring node.
    pub async fn resolve(&self, did: CoreDID) -> anyhow::Result<Option<CoreDocument>> {
        let mut split = did.method_id().split(':');
        let alias_id: AliasId = AliasId::new(prefix_hex::decode(split.next().unwrap()).unwrap());
        let did_tag = split.next().unwrap();

        let output = self.resolve_alias_output(alias_id).await?;

        let content: AliasContent = AliasContent::from_json_slice(output.2.state_metadata())?;
        let index: DIDIndex = self.storage.get_index(&content.index_cid).await?;

        let did = IotaDID::parse(format!("did:iota:{did_tag}")).unwrap();

        match self.storage.get(&did, &index).await? {
            Some(coc) => {
                let serialized = coc.chain_of_custody.serialize_to_vec()?;
                let document = coc.chain_of_custody.into_document()?;

                log::debug!("verifying the proof for {did}");

                if !coc.proof.verify(&content.merkle_root, serialized) {
                    anyhow::bail!("invalid merkle proof for {did}");
                }

                Ok(Some(document))
            }
            None => Ok(None),
        }
    }

    async fn resolve_alias_output(
        &self,
        alias_id: AliasId,
    ) -> anyhow::Result<(AliasId, OutputId, AliasOutput)> {
        let output_id: OutputId = self.client.alias_output_id(alias_id).await?;
        let output_response: OutputResponse = self.client.get_output(&output_id).await?;
        let output: Output = Output::try_from(&output_response.output)?;

        if let Output::Alias(alias_output) = output {
            Ok((alias_id, output_id, alias_output))
        } else {
            unreachable!("we requested an alias output. (TODO: turn into error later, though.)");
        }
    }
}
