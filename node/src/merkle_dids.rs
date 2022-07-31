use std::collections::{hash_map::Entry, HashMap};

use crypto::hashes::blake2b::Blake2b256;
use identity_iota_client::{chain::IntegrationChain, document::ResolvedIotaDocument};
use identity_iota_core::did::IotaDID;
use merkle_tree::{MerkleTree, Proof};

use crate::{ChainOfCustody, IndexedChainOfCustody, SerializedChainOfCustody};

pub struct MerkleDIDs {
    merkle_tree: MerkleTree<SerializedChainOfCustody>,
    document_tree: HashMap<IotaDID, IndexedChainOfCustody>,
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
                let mut iterator = entry.get().chain_of_custody.0.iter();

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

                entry.get_mut().chain_of_custody.0.push(document);

                // Update Merkle Tree.
                // Serialize the entire chain of custody.

                let serialized = entry.get().chain_of_custody.serialize_to_vec()?;

                let index: usize = entry.get().merkle_tree_index;

                self.merkle_tree.replace(index, serialized);
            }
            Entry::Vacant(entry) => {
                // Make sure it's a valid root document.
                IntegrationChain::new(document.clone())?;

                let chain_of_custody: ChainOfCustody = ChainOfCustody(vec![document]);

                let serialized: SerializedChainOfCustody = chain_of_custody.serialize_to_vec()?;

                let merkle_tree_index: usize = self.merkle_tree.push(serialized);

                entry.insert(IndexedChainOfCustody {
                    chain_of_custody,
                    merkle_tree_index,
                });
            }
        }

        Ok(())
    }

    pub fn merkle_root(&self) -> Vec<u8> {
        self.merkle_tree.root::<Blake2b256>()
    }

    pub fn generate_merkle_proof(&self, did: &IotaDID) -> Option<Proof<Blake2b256>> {
        let entry: &IndexedChainOfCustody = self.document_tree.get(did)?;
        self.merkle_tree
            .generate_proof::<Blake2b256>(entry.merkle_tree_index)
    }

    pub fn chain_of_custody(&self, did: &IotaDID) -> Option<&ChainOfCustody> {
        self.document_tree.get(did).map(|coc| &coc.chain_of_custody)
    }
}

#[cfg(test)]
mod tests {
    use identity_core::{
        common::Url,
        crypto::{KeyPair, KeyType},
    };
    use identity_did::{did::DID, verification::MethodScope};
    use identity_iota_client::{document::ResolvedIotaDocument, tangle::TangleRef};
    use identity_iota_core::{
        did::IotaDID,
        document::{IotaDocument, IotaService, IotaVerificationMethod},
        tangle::MessageId,
    };

    use super::MerkleDIDs;

    fn gen_document() -> (KeyPair, ResolvedIotaDocument) {
        let keypair: KeyPair = KeyPair::new(KeyType::Ed25519).unwrap();

        let mut document: IotaDocument = IotaDocument::new(&keypair).unwrap();

        document
            .sign_self(
                keypair.private(),
                document.default_signing_method().unwrap().id().clone(),
            )
            .unwrap();

        let mut doc = ResolvedIotaDocument::from(document);
        doc.set_message_id(random_message_id());

        (keypair, doc)
    }

    fn service(did: &IotaDID, fragment: &str, type_: &str, endpoint: &str) -> IotaService {
        IotaService::builder(Default::default())
            .id(did.to_url().join(fragment).unwrap())
            .type_(type_)
            .service_endpoint(Url::parse(endpoint).unwrap().into())
            .build()
            .unwrap()
    }

    fn random_message_id() -> MessageId {
        MessageId::new(rand::random())
    }

    fn update_document<F>(
        keypair: &KeyPair,
        mut doc: ResolvedIotaDocument,
        f: F,
    ) -> ResolvedIotaDocument
    where
        F: FnOnce(&mut IotaDocument),
    {
        f(&mut doc.document);

        doc.document.metadata.previous_message_id = doc.message_id().clone();

        doc.document
            .sign_self(
                keypair.private(),
                doc.document.default_signing_method().unwrap().id().clone(),
            )
            .unwrap();

        doc.set_message_id(random_message_id());

        doc
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

        let doc = update_document(&keypair, doc, |document| {
            document.insert_service(service(
                document.id(),
                "#my-service",
                "MyServiceType",
                "http://example.com/service/",
            ));
        });

        merkle_dids.update_document(doc).unwrap();
    }

    #[test]
    fn test_merkle_dids_rotate_keys() {
        let (keypair, document) = gen_document();

        let mut doc = ResolvedIotaDocument::from(document);
        let doc_message_id = random_message_id();
        doc.set_message_id(doc_message_id);

        let mut merkle_dids = MerkleDIDs::new();

        merkle_dids.update_document(doc.clone()).unwrap();

        let keypair2 = KeyPair::new(KeyType::Ed25519).unwrap();

        let mut doc = update_document(&keypair, doc, |document| {
            let method: IotaVerificationMethod = IotaVerificationMethod::new(
                document.id().to_owned(),
                keypair2.type_(),
                keypair2.public(),
                "#key-2",
            )
            .unwrap();

            document
                .insert_method(method, MethodScope::capability_invocation())
                .unwrap();
        });

        merkle_dids.update_document(doc.clone()).unwrap();

        doc.document
            .remove_method(
                &doc.document
                    .id()
                    .to_url()
                    .join(format!("#{}", IotaDocument::DEFAULT_METHOD_FRAGMENT))
                    .unwrap(),
            )
            .unwrap();

        doc.document.metadata.previous_message_id = doc.message_id().clone();

        doc.document
            .sign_self(
                keypair2.private(),
                doc.document.id().to_url().join("#key-2").unwrap(),
            )
            .unwrap();

        doc.set_message_id(random_message_id());

        merkle_dids.update_document(doc).unwrap();
    }

    #[test]
    fn test_merkle_dids_gen_proof() {
        let (_keypair1, document1) = gen_document();
        let (_keypair2, document2) = gen_document();
        let (keypair3, document3) = gen_document();
        let (_keypair4, document4) = gen_document();

        let mut merkle_dids = MerkleDIDs::new();

        merkle_dids.update_document(document1).unwrap();
        merkle_dids.update_document(document2).unwrap();
        merkle_dids.update_document(document3.clone()).unwrap();
        merkle_dids.update_document(document4).unwrap();

        let document3 = update_document(&keypair3, document3, |document| {
            let service = IotaService::builder(Default::default())
                .id(document.id().to_url().join("#my-service-3").unwrap())
                .type_("AServiceType")
                .service_endpoint(
                    Url::parse("http://example.com/service.json")
                        .unwrap()
                        .into(),
                )
                .build()
                .unwrap();

            document.insert_service(service);
        });

        merkle_dids.update_document(document3.clone()).unwrap();

        let document3_proof = merkle_dids.generate_merkle_proof(document3.did()).unwrap();

        let coc = merkle_dids.chain_of_custody(document3.did()).unwrap();
        let coc_serialized = coc.serialize_to_vec().unwrap();

        assert!(document3_proof.verify(merkle_dids.merkle_root().as_ref(), coc_serialized))
    }
}
