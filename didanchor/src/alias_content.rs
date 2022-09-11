#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AliasContent {
    pub index_cid: String,
    pub ipfs_node_addrs: Vec<IpfsNodeAddress>,
    pub merkle_root: Vec<u8>,
}

impl AliasContent {
    pub fn new(
        index_cid: String,
        ipfs_node_addrs: Vec<IpfsNodeAddress>,
        merkle_root: Vec<u8>,
    ) -> Self {
        Self {
            index_cid,
            ipfs_node_addrs,
            merkle_root,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IpfsNodeAddress {
    pub host: String,
    pub swarm_port: u16,
    pub gateway_port: u16,
    pub cluster_port: u16,
    pub peer_id: String,
}
