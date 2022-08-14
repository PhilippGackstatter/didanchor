use bytes::Bytes;
use rand::Rng;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct IpfsGateway {
    http_client: Client,
    ipfs_gateway_addrs: Vec<String>,
}

impl IpfsGateway {
    pub fn new(ipfs_gateway_addrs: Vec<String>) -> Self {
        Self {
            http_client: Client::new(),
            ipfs_gateway_addrs,
        }
    }

    pub async fn get(&self, cid: &str) -> anyhow::Result<Bytes> {
        // Pick a random gateway.
        let endpoint: &str = &self.ipfs_gateway_addrs
            [rand::thread_rng().gen_range(0..self.ipfs_gateway_addrs.len())];

        let url = format!("{endpoint}/ipfs/{cid}");

        let request = self.http_client.get(url.clone()).build()?;

        let response = self.http_client.execute(request).await?;

        if response.status().is_success() {
            response.bytes().await.map_err(Into::into)
        } else {
            Err(anyhow::anyhow!(
                "error response for GET {url}: HTTP Status {}",
                response.status()
            ))
        }
    }
}
