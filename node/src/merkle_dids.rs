use std::collections::{hash_map::Entry, HashMap};

use crypto::hashes::blake2b::Blake2b256;
use identity_iota_client::{chain::IntegrationChain, document::ResolvedIotaDocument};
use identity_iota_core::did::IotaDID;
use merkle_tree::MerkleTree;

#[derive(Debug, Clone, Default)]
pub struct ChainOfCustody(Vec<ResolvedIotaDocument>);

#[derive(Debug, Clone, Default)]
pub struct ChainOfCustodySerialized(Vec<u8>);

impl AsRef<[u8]> for ChainOfCustodySerialized {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

pub struct MerkleDIDs {
    merkle_tree: MerkleTree<ChainOfCustodySerialized>,
    document_tree: HashMap<IotaDID, ChainOfCustody>,
}

impl MerkleDIDs {
    pub fn new() -> Self {
        Self {
            merkle_tree: MerkleTree::new(),
            document_tree: HashMap::new(),
        }
    }

    /// Updates the document or inserts it if it doesn't exist.
    pub fn update_document(&mut self, document: ResolvedIotaDocument) -> anyhow::Result<()> {
        match self.document_tree.entry(document.document.id().to_owned()) {
            Entry::Occupied(mut entry) => {
                let mut iterator = entry.get().0.iter();

                let mut chain = IntegrationChain::new(
                    iterator
                        .next()
                        .expect("non-empty vectors should never be inserted")
                        .to_owned(),
                )?;

                // Doing this validation every time is unnecessary,
                // but there's no (public) API to start a chain from a non-root document (afaik).
                for elem in iterator {
                    chain.try_push(elem.to_owned())?;
                }

                chain.check_valid_addition(&document)?;

                entry.get_mut().0.push(document);
            }
            Entry::Vacant(entry) => {
                // Make sure it's a valid root document.
                IntegrationChain::new(document.clone())?;

                entry.insert(ChainOfCustody(vec![document]));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use identity_core::{
        common::Url,
        crypto::{KeyPair, KeyType},
    };
    use identity_did::did::DID;
    use identity_iota_client::{document::ResolvedIotaDocument, tangle::TangleRef};
    use identity_iota_core::{
        document::{IotaDocument, IotaService},
        tangle::MessageId,
    };

    use super::MerkleDIDs;

    fn gen_document() -> (KeyPair, IotaDocument) {
        let keypair: KeyPair = KeyPair::new(KeyType::Ed25519).unwrap();

        let mut document: IotaDocument = IotaDocument::new(&keypair).unwrap();

        document
            .sign_self(
                keypair.private(),
                document.default_signing_method().unwrap().id().clone(),
            )
            .unwrap();

        (keypair, document)
    }

    fn random_message_id() -> MessageId {
        MessageId::new(rand::random())
    }

    #[test]
    fn test_merkle_dids_create_document() {
        let (_keypair, document) = gen_document();

        let mut doc = ResolvedIotaDocument::from(document);
        doc.set_message_id(random_message_id());

        let mut merkle_dids = MerkleDIDs::new();

        merkle_dids.update_document(doc).unwrap();
    }

    #[test]
    fn test_merkle_dids_update_document() {
        let (keypair, document) = gen_document();

        let mut doc = ResolvedIotaDocument::from(document);
        let doc_message_id = random_message_id();
        doc.set_message_id(doc_message_id);

        let mut merkle_dids = MerkleDIDs::new();

        merkle_dids.update_document(doc.clone()).unwrap();

        let service = IotaService::builder(Default::default())
            .id(doc.document.id().to_url().join("#my-service").unwrap())
            .type_("MyServiceType")
            .service_endpoint(Url::parse("http://example.com/service/").unwrap().into())
            .build()
            .unwrap();

        doc.document.insert_service(service);
        doc.document.metadata.previous_message_id = doc.message_id().clone();

        doc.document
            .sign_self(
                keypair.private(),
                doc.document.default_signing_method().unwrap().id().clone(),
            )
            .unwrap();

        doc.set_message_id(random_message_id());

        merkle_dids.update_document(doc).unwrap();
    }
}
