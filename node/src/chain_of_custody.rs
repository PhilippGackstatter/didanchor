use identity_core::convert::ToJson;
use identity_iota_client::document::ResolvedIotaDocument;

/// A chain of DID updates that can be verified independently.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ChainOfCustody(pub Vec<ResolvedIotaDocument>);

impl ChainOfCustody {
    // TODO: Only used for hashing, could take a Digest instance directly and digest bytes incrementally.
    pub fn serialize_to_vec(&self) -> anyhow::Result<SerializedChainOfCustody> {
        let mut serialized = Vec::new();

        for doc in self.0.iter() {
            let bytes = doc.to_json_vec()?;
            serialized.extend(bytes);
        }

        Ok(serialized)
    }
}

pub type SerializedChainOfCustody = Vec<u8>;
