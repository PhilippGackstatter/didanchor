use identity_core::convert::ToJson;
use identity_did::document::CoreDocument;
use identity_iota_client::{chain::IntegrationChain, document::ResolvedIotaDocument};

/// A chain of DID updates that can be verified independently.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChainOfCustody(pub Vec<ResolvedIotaDocument>);

impl ChainOfCustody {
    // TODO: Only used for hashing, could take a Digest instance directly and digest bytes incrementally.
    pub fn serialize_to_vec(&self) -> anyhow::Result<Vec<u8>> {
        let mut serialized = Vec::new();

        for doc in self.0.iter() {
            let bytes = doc.to_json_vec()?;
            serialized.extend(bytes);
        }

        Ok(serialized)
    }

    pub fn into_document(self) -> anyhow::Result<CoreDocument> {
        let mut iterator = self.0.iter();

        let first = if let Some(first) = iterator.next() {
            first
        } else {
            anyhow::bail!("expected at least one entry in the chain");
        };

        let mut chain = IntegrationChain::new(first.to_owned())?;

        for elem in iterator {
            chain.try_push(elem.to_owned())?;
        }

        Ok(chain
            .current()
            .document
            .core_document()
            .to_owned()
            .map(|did| did.into(), |g| g))
    }
}
