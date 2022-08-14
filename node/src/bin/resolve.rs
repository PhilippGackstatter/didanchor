use identity_did::did::CoreDID;
use identity_iota_core::did::IotaDID;
use node::AnchorConfig;
use node::Resolver;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AnchorConfig::read_default_location().await?;

    let iota_did = IotaDID::parse("did:iota:ChSJ2aM4V31CNCRnVbmGh7hU1hqbZ36ZSoPbArso5Y6N").unwrap();
    let alias_id = config.alias_id;

    // TODO: Should be obtained from the output.
    let node_addr = config.ipfs_node_addrs.into_iter().next().unwrap();

    let did = CoreDID::parse(format!("did:iota:{alias_id}:{}", iota_did.tag())).unwrap();

    let (hostname, port) = { (node_addr.0, node_addr.1.parse::<u16>().unwrap()) };

    println!("retrieving from IPFS node {hostname}:{port}");

    let resolver = Resolver::new(&hostname, port)?;

    let document = resolver.resolve(did).await?;

    println!("Resolved document {document:#?}");

    Ok(())
}
