use reqwest::{
    multipart::{Form, Part},
    Client,
};

use crate::AddResponse;

#[derive(Debug, Clone)]
pub struct IpfsClusterClient {
    client: Client,
    hostname: String,
}

impl IpfsClusterClient {
    pub fn new() -> Self {
        let client = Client::new();

        Self {
            client,
            hostname: "http://127.0.0.1:9094".to_owned(),
        }
    }

    pub fn new_with_host(hostname: impl Into<String>) -> Self {
        let client = Client::new();

        Self {
            client,
            hostname: hostname.into(),
        }
    }

    /// Add `data` to the cluster and pin it to every peer.
    pub async fn add(&self, data: Vec<u8>) -> anyhow::Result<AddResponse> {
        let endpoint = format!("{}/add", self.hostname);

        // The name doesn't matter.
        let form = Form::new().part("_data", Part::bytes(data));
        let request = self
            .client
            .post(endpoint)
            .multipart(form)
            // -1 means pinning it to every peer in the cluster.
            .query(&[("replication-min", "-1"), ("replication-max", "-1")])
            .build()?;

        log::debug!("{request:?}");

        let response = self.client.execute(request).await?;
        let add_response: AddResponse = response.json().await?;

        Ok(add_response)
    }

    /// Unpins the given `cid` from the cluster.
    pub async fn unpin(&self, cid: &str) -> anyhow::Result<()> {
        let endpoint = format!("{}/pins/ipfs/{}", self.hostname, cid);

        let request = self.client.delete(endpoint).build()?;

        log::debug!("{request:?}");

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

impl Default for IpfsClusterClient {
    fn default() -> Self {
        Self::new()
    }
}
