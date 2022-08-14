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

use crate::{AliasContent, ChainStorage, DIDIndex};

pub struct Resolver {
    client: Client,
}

impl Resolver {
    pub fn new(iota_endpoint: &str) -> anyhow::Result<Self> {
        let client: Client = Client::builder()
            .with_primary_node(iota_endpoint, None)?
            .finish()?;

        Ok(Self { client })
    }

    /// Resolve the given did into its corresponding DID document.
    ///
    /// Ensures validity in the chain of custody, as well as ensuring it is the version of the CoC
    /// committed to by the anchoring node.
    pub async fn resolve(&self, did: &CoreDID) -> anyhow::Result<Option<CoreDocument>> {
        let mut split = did.method_id().split(':');
        let alias_id: AliasId = AliasId::new(prefix_hex::decode(split.next().unwrap()).unwrap());
        let did_tag = split.next().unwrap();

        let content: AliasContent = resolve_alias_content(&self.client, alias_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("no output found for alias id {alias_id}"))?;

        let chain_storage = ChainStorage::new(content.ipfs_gateway_addrs);
        let index: DIDIndex = chain_storage.get_index(&content.index_cid).await?;

        let did = IotaDID::parse(format!("did:iota:{did_tag}")).unwrap();

        match chain_storage.get(&did, &index).await? {
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
}

pub(crate) async fn resolve_alias_content(
    client: &Client,
    alias_id: AliasId,
) -> anyhow::Result<Option<AliasContent>> {
    let (_, _, alias_output) = if let Some(output) = resolve_alias_output(client, alias_id).await? {
        output
    } else {
        return Ok(None);
    };

    let content: AliasContent = AliasContent::from_json_slice(alias_output.state_metadata())?;
    Ok(Some(content))
}

/// Resolve a did into an Alias Output and the associated identifiers.
pub(crate) async fn resolve_alias_output(
    client: &Client,
    alias_id: AliasId,
) -> anyhow::Result<Option<(AliasId, OutputId, AliasOutput)>> {
    let output_id: OutputId = match client.alias_output_id(alias_id).await {
        Ok(output_id) => output_id,
        Err(iota_client::Error::NotFound) => return Ok(None),
        Err(err) => anyhow::bail!(err),
    };

    let output_response: OutputResponse = client.get_output(&output_id).await?;
    let output: Output = Output::try_from(&output_response.output)?;

    if let Output::Alias(alias_output) = output {
        Ok(Some((alias_id, output_id, alias_output)))
    } else {
        unreachable!("we requested an alias output. (TODO: turn into error later, though.)");
    }
}
