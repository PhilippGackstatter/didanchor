use crate::AddResponse;
use rand::Rng;
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use url::Url;

#[derive(Debug, Clone)]
pub struct IpfsCluster {
    client: Client,
    node_addrs: Vec<Url>,
}

impl IpfsCluster {
    pub fn new(node_addrs: Vec<Url>) -> anyhow::Result<Self> {
        if node_addrs.is_empty() {
            anyhow::bail!("`node_addrs` cannot be empty");
        }

        let client: Client = Client::new();

        Ok(Self { client, node_addrs })
    }

    pub fn get_random_node(&self) -> &Url {
        // Indexing is fine, since we assert in the constructor that the collection is not empty.
        &self.node_addrs[rand::thread_rng().gen_range(0..self.node_addrs.len())]
    }

    /// Add `data` to the cluster and pin it to every peer.
    pub async fn add(&self, data: Vec<u8>) -> anyhow::Result<AddResponse> {
        let node_url: &Url = self.get_random_node();
        let endpoint: Url = node_url.join("add")?;

        // The name doesn't matter.
        let form = Form::new().part("_data", Part::bytes(data));
        let request = self
            .client
            .post(endpoint)
            .multipart(form)
            // -1 means pinning it to every peer in the cluster.
            .query(&[
                ("replication-min", "-1"),
                ("replication-max", "-1"),
                ("hash", "blake2b-256"),
                ("cid-version", "1"),
            ])
            .build()?;

        log::trace!("{request:?}");

        let response = self.client.execute(request).await?;
        let add_response: AddResponse = response.json().await?;

        Ok(add_response)
    }

    /// Unpins the given `cid` from the cluster.
    pub async fn unpin(&self, cid: &str) -> anyhow::Result<()> {
        let node_url: &Url = self.get_random_node();
        let endpoint: Url = node_url.join("pins/ipfs/")?.join(cid)?;

        let request = self.client.delete(endpoint).build()?;

        log::trace!("{request:?}");

        let response = self.client.execute(request).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "failed to unpin with status {}",
                response.status()
            ))
        }
    }
}
