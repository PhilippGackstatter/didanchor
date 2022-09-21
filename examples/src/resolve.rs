use std::time::Instant;

use didanchor::{AnchorConfig, Resolver};
use identity_core::convert::ToJson;
use identity_did::did::CoreDID;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();

    let did: String = args.nth(1).ok_or_else(|| {
        anyhow::anyhow!("expected a `did:iota:<alias_id>:<tag>` as the first argument")
    })?;

    let did: CoreDID = CoreDID::parse(did)?;

    let config: AnchorConfig = AnchorConfig::read_default_location().await?;

    let time = Instant::now();

    let resolver = Resolver::new(&config.iota_endpoint, "http://127.0.0.1:5001")?;

    match resolver.resolve(&did).await? {
        Some(document) => {
            println!("{}", document.to_json_pretty()?);
            println!("Resolution took {}ms", time.elapsed().as_millis());
        }
        None => {
            println!("Unable to resolve {did}");
        }
    }

    Ok(())
}
