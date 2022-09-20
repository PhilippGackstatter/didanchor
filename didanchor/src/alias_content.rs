use crate::IpfsNodePublicAddress;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AliasContent {
    pub index_cid: String,
    pub ipfs_node_addrs: Vec<IpfsNodePublicAddress>,
    pub merkle_root: Vec<u8>,
}

impl AliasContent {
    pub fn new(
        index_cid: String,
        ipfs_node_addrs: Vec<IpfsNodePublicAddress>,
        merkle_root: Vec<u8>,
    ) -> Self {
        Self {
            index_cid,
            ipfs_node_addrs,
            merkle_root,
        }
    }
}
