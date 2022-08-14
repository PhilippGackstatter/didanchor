use didanchor::Resolver;
use identity_core::convert::ToJson;
use identity_did::did::CoreDID;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();

    let did: String = args.nth(1).ok_or_else(|| {
        anyhow::anyhow!("expected a `did:iota:<alias_id>:<tag>` as the first argument")
    })?;

    let did = CoreDID::parse(did)?;

    let resolver = Resolver::new()?;

    match resolver.resolve(&did).await? {
        Some(document) => {
            println!("{}", document.to_json_pretty()?);
        }
        None => {
            println!("Unable to resolve {did}");
        }
    }

    Ok(())
}
