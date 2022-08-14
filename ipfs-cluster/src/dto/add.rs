#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddResponse {
    pub name: String,
    pub cid: String,
    pub size: u64,
    pub allocations: Vec<String>,
}
