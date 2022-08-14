#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

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
use didanchor::Anchor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let mut node = Anchor::new().await?;

    let (keypair1, doc1) = gen_document();
    let (keypair2, doc2) = gen_document();
    let (keypair3, doc3) = gen_document();
    let (keypair4, doc4) = gen_document();

    node.update_document(doc1.clone()).await?;
    node.update_document(doc2).await?;
    node.update_document(doc3).await?;
    node.update_document(doc4).await?;

    // node.commit_changes().await?;

    let doc1 = update_document(&keypair1, doc1, |doc| {
        doc.insert_service(service(
            doc.id(),
            "#my-service",
            "AnchorService",
            "http://ipfs.com",
        ));
    });

    node.update_document(doc1.clone()).await?;

    let doc1 = update_document(&keypair1, doc1, |doc| {
        doc.insert_method(method(doc.id(), "#key-2"), MethodScope::authentication())
            .unwrap();
    });

    node.update_document(doc1.clone()).await?;

    node.commit_changes().await?;

    println!("updated {} several times", doc1.document.id());

    Ok(())
}

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

fn random_message_id() -> MessageId {
    MessageId::new(rand::random())
}

fn service(did: &IotaDID, fragment: &str, type_: &str, endpoint: &str) -> IotaService {
    IotaService::builder(Default::default())
        .id(did.to_url().join(fragment).unwrap())
        .type_(type_)
        .service_endpoint(Url::parse(endpoint).unwrap().into())
        .build()
        .unwrap()
}

fn method(did: &IotaDID, fragment: &str) -> IotaVerificationMethod {
    let keypair = KeyPair::new(KeyType::Ed25519).unwrap();
    IotaVerificationMethod::new(did.to_owned(), KeyType::Ed25519, keypair.public(), fragment)
        .unwrap()
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

    doc.document.metadata.previous_message_id = *doc.message_id();

    doc.document
        .sign_self(
            keypair.private(),
            doc.document.default_signing_method().unwrap().id().clone(),
        )
        .unwrap();

    doc.set_message_id(random_message_id());

    doc
}
