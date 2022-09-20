use anyhow::Context;
use bytes::Bytes;
use did_common::VerifiableChainOfCustody;
use identity_core::convert::FromJson;
use identity_did::{
    did::{CoreDID, DID},
    document::CoreDocument,
};
use identity_iota_core::did::IotaDID;
use iota_client::{
    api_types::responses::OutputResponse,
    block::output::{AliasId, AliasOutput, Output, OutputId},
    Client as IotaClient,
};
use ipfs_client::IpfsClient;
use multiaddr::Multiaddr;
use packable::{unpacker::SliceUnpacker, Packable};

use crate::{AliasContent, DIDIndex};

pub struct Resolver {
    iota_client: IotaClient,
    ipfs_client: IpfsClient,
}

impl Resolver {
    pub fn new(iota_endpoint: &str, ipfs_endpoint: &str) -> anyhow::Result<Self> {
        let client: IotaClient = IotaClient::builder()
            .with_primary_node(iota_endpoint, None)?
            .finish()?;

        let ipfs_client = IpfsClient::new(vec![ipfs_endpoint
            .parse()
            .context("unable to parse IPFS endpoint into URL")?])?;

        Ok(Self {
            iota_client: client,
            ipfs_client,
        })
    }

    /// Resolve the given DID into its corresponding DID document.
    ///
    /// Ensures validity in the chain of custody, as well as ensuring it is the version of the CoC
    /// committed to by the anchoring node.
    pub async fn resolve(&self, did: &CoreDID) -> anyhow::Result<Option<CoreDocument>> {
        let mut split = did.method_id().split(':');
        let alias_id: AliasId = AliasId::new(prefix_hex::decode(split.next().unwrap()).unwrap());
        let did_tag = split.next().unwrap();
        let did = IotaDID::parse(format!("did:iota:{did_tag}")).unwrap();

        let alias_content: AliasContent = resolve_alias_content(&self.iota_client, alias_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("no output found for alias id {alias_id}"))?;

        self.resolve_did(&alias_content, &did).await
    }

    async fn resolve_did(
        &self,
        content: &AliasContent,
        did: &IotaDID,
    ) -> anyhow::Result<Option<CoreDocument>> {
        // Peer the local node with a cluster node to speed up the subsequent lookup.
        let node_multiaddrs = content.ipfs_node_addrs.iter().map(|addr| {
            let mut multiaddr = Multiaddr::empty();
            multiaddr.push(addr.host.clone());

            for protocol in addr.swarm_port.iter() {
                multiaddr.push(protocol);
            }

            multiaddr.push(addr.peer_id.clone());
            multiaddr
        });

        // TODO: Poll all swarm connect futures at once, or consider peering with just one random node.
        for addr in node_multiaddrs {
            self.ipfs_client.swarm_connect(addr.to_string()).await?;
        }

        let index_bytes: Bytes = self.ipfs_client.cat(&content.index_cid).await?;
        let index: DIDIndex = DIDIndex::from_json_slice(&index_bytes)?;

        let cid: &str = if let Some(cid) = index.get(did) {
            cid
        } else {
            return Ok(None);
        };

        let bytes: Bytes = self.ipfs_client.cat(cid).await?;

        let mut unpacker = SliceUnpacker::new(bytes.as_ref());
        let coc: VerifiableChainOfCustody =
            VerifiableChainOfCustody::unpack::<_, false>(&mut unpacker).expect("TODO");

        let serialized = coc.chain_of_custody.serialize_to_vec()?;
        let document = coc.chain_of_custody.into_document()?;

        log::debug!("verifying the proof for {did}");

        if !coc.proof.verify(&content.merkle_root, serialized) {
            anyhow::bail!("invalid merkle proof for {did}");
        }

        Ok(Some(document))
    }
}

pub(crate) async fn resolve_alias_content(
    client: &IotaClient,
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
    client: &IotaClient,
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
