use futures::TryStreamExt;
use http::uri::Scheme;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient as Client, TryFromUri};
use url::Url;

#[derive(Clone)]
pub struct IpfsClient {
    client: Client,
    // TODO: For now only one address is used, but eventually these could be round-robined in retries.
    _node_addrs: Vec<Url>,
}

impl IpfsClient {
    pub fn new(node_addrs: Vec<Url>) -> anyhow::Result<Self> {
        if node_addrs.is_empty() {
            anyhow::bail!("`node_addrs` cannot be empty");
        }

        let client: Client = Client::from_host_and_port(
            Scheme::HTTP,
            node_addrs[0]
                .host_str()
                .ok_or_else(|| anyhow::anyhow!("expected a valid hostname"))?,
            node_addrs[0]
                .port()
                .ok_or_else(|| anyhow::anyhow!("expected a port"))?,
        )?;

        Ok(Self {
            client,
            _node_addrs: node_addrs,
        })
    }

    pub async fn get_bytes(&self, cid: &str) -> anyhow::Result<Vec<u8>> {
        Ok(self
            .client
            .cat(cid)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
            .await?)
    }
}
