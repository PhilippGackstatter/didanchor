use anyhow::Context;
use rand::Rng;
use reqwest::{Client, Response};
use url::Url;

#[derive(Clone)]
pub struct IpfsClient {
    client: Client,
    node_addrs: Vec<Url>,
}

impl IpfsClient {
    pub fn new(node_addrs: Vec<Url>) -> anyhow::Result<Self> {
        if node_addrs.is_empty() {
            anyhow::bail!("`node_addrs` cannot be empty");
        }

        let client: Client = Client::new();

        Ok(Self {
            client,
            node_addrs: node_addrs,
        })
    }

    pub fn get_random_node(&self) -> &Url {
        // Indexing is fine, since we assert in the constructor that the collection is not empty.
        &self.node_addrs[rand::thread_rng().gen_range(0..self.node_addrs.len())]
    }

    /// Open connection to a given address.
    ///
    /// Example address: `/ip4/127.0.0.1/udp/4001/quic/p2p/12D3KooWL3EovpbdH1Axsk51xv9ascEsv9a81BuQdSZyNDtRSaHu`
    ///
    /// In particular, the address needs to include the Peer Id.
    ///
    /// <https://docs.ipfs.tech/reference/kubo/rpc/#api-v0-swarm-connect>
    pub async fn swarm_connect(&self, addr: impl AsRef<str>) -> anyhow::Result<()> {
        let node_url: &Url = self.get_random_node();
        let endpoint: Url = node_url.join("api/v0/swarm/connect")?;

        let request = self
            .client
            .post(endpoint)
            .query(&[("arg", addr.as_ref())])
            .build()?;

        self.execute_request(request)
            .await
            .context("adding peer failed")?;

        Ok(())
    }

    /// Show IPFS object data.
    ///
    /// <https://docs.ipfs.tech/reference/kubo/rpc/#api-v0-cat>
    pub async fn cat(&self, cid: &str) -> anyhow::Result<bytes::Bytes> {
        let node_url: &Url = self.get_random_node();
        let endpoint: Url = node_url.join("api/v0/cat")?;

        let request = self
            .client
            .post(endpoint)
            .query(&[("arg", cid), ("progress", "false")])
            .build()?;

        let response = self.execute_request(request).await.context("cat failed")?;

        Ok(response.bytes().await?)
    }

    async fn execute_request(&self, request: reqwest::Request) -> anyhow::Result<Response> {
        let response: Response = self.client.execute(request).await?;

        if response.status().is_success() {
            Ok(response)
        } else {
            anyhow::bail!("{}", response.status())
        }
    }
}
