use identity_core::convert::ToJson;
use iota_client::{
    block::{
        address::Address,
        output::{
            unlock_condition::{
                GovernorAddressUnlockCondition, StateControllerAddressUnlockCondition,
            },
            AliasId, AliasOutput, AliasOutputBuilder, Output, OutputId, RentStructure,
            UnlockCondition,
        },
        payload::{transaction::TransactionEssence, Payload},
        Block,
    },
    secret::{mnemonic::MnemonicSecretManager, SecretManager},
    Client,
};

use crate::resolve_alias_output;

pub static IOTA_NETWORK_ENDPOINT: &str = "https://api.alphanet.iotaledger.net";
// static FAUCET_URL: &str = "https://faucet.alphanet.iotaledger.net/api/enqueue";

#[derive(Debug)]
pub struct AnchorOutput {
    pub(crate) client: Client,
    pub(crate) alias_id: AliasId,
    // insecure.
    secret_manager: SecretManager,
}

impl AnchorOutput {
    pub fn new(mnemonic: String, alias_id: AliasId) -> anyhow::Result<Self> {
        let client: Client = Client::builder()
            .with_primary_node(IOTA_NETWORK_ENDPOINT, None)?
            .finish()?;

        let secret_manager: SecretManager =
            SecretManager::Mnemonic(MnemonicSecretManager::try_from_mnemonic(&mnemonic)?);

        Ok(AnchorOutput {
            client,
            alias_id,
            secret_manager,
        })
    }

    pub async fn publish_output(&mut self, content: AliasContent) -> anyhow::Result<AliasId> {
        log::debug!("publishing new Alias Output");

        let content_vec = content.to_json_vec()?;

        let rent_structure = self.client.get_rent_structure().await?;

        let alias_output: AliasOutput = if self.alias_id.is_null() {
            self.new_output(content_vec, rent_structure).await?
        } else {
            self.update_output(content_vec, rent_structure, self.alias_id)
                .await?
        };

        let block: Block = self
            .client
            .block()
            .with_secret_manager(&self.secret_manager)
            .with_outputs(vec![alias_output.into()])?
            .finish()
            .await?;

        let _ = self
            .client
            .retry_until_included(&block.id(), None, None)
            .await?;

        let alias_id = Self::alias_ids_from_block(&block)?
            .into_iter()
            .next()
            .expect("there should be exactly one alias id");

        log::debug!("published output with id {alias_id}");

        self.alias_id = alias_id;

        Ok(alias_id)
    }

    async fn new_output(
        &self,
        state_metadata: Vec<u8>,
        rent_structure: RentStructure,
    ) -> anyhow::Result<AliasOutput> {
        let address: Address = self
            .client
            .get_addresses(&self.secret_manager)
            .with_range(0..1)
            .get_raw()
            .await?[0];

        Ok(
            AliasOutputBuilder::new_with_minimum_storage_deposit(rent_structure, AliasId::null())?
                .with_state_index(0)
                .with_foundry_counter(0)
                .with_state_metadata(state_metadata)
                // .add_feature(Feature::Sender(SenderFeature::new(address)))
                .add_unlock_condition(UnlockCondition::StateControllerAddress(
                    StateControllerAddressUnlockCondition::new(address),
                ))
                .add_unlock_condition(UnlockCondition::GovernorAddress(
                    GovernorAddressUnlockCondition::new(address),
                ))
                .finish()?,
        )
    }

    async fn update_output(
        &self,
        state_metadata: Vec<u8>,
        rent_structure: RentStructure,
        alias_id: AliasId,
    ) -> anyhow::Result<AliasOutput> {
        let (alias_id, _, alias_output) = resolve_alias_output(&self.client, alias_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("no output found for alias id {alias_id}"))?;

        let mut alias_output_builder: AliasOutputBuilder = AliasOutputBuilder::from(&alias_output)
            .with_minimum_storage_deposit(rent_structure)
            .with_state_index(alias_output.state_index() + 1)
            .with_state_metadata(state_metadata);

        if alias_output.alias_id().is_null() {
            alias_output_builder = alias_output_builder.with_alias_id(alias_id);
        }

        Ok(alias_output_builder.finish()?)
    }

    /// Returns all DID documents of the Alias Outputs contained in the payload's transaction, if any.
    fn alias_ids_from_block(block: &Block) -> anyhow::Result<Vec<AliasId>> {
        let mut documents = Vec::new();

        if let Some(Payload::Transaction(tx_payload)) = block.payload() {
            let TransactionEssence::Regular(regular) = tx_payload.essence();

            for (index, output) in regular.outputs().iter().enumerate() {
                if let Output::Alias(alias_output) = output {
                    let alias_id = if alias_output.alias_id().is_null() {
                        AliasId::from(OutputId::new(tx_payload.id(), index.try_into()?)?)
                    } else {
                        alias_output.alias_id().to_owned()
                    };

                    documents.push(alias_id);
                }
            }
        }

        Ok(documents)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AliasContent {
    pub index_cid: String,
    pub ipfs_gateway_addrs: Vec<String>,
    pub merkle_root: Vec<u8>,
}

impl AliasContent {
    pub fn new(index_cid: String, ipfs_gateway_addrs: Vec<String>, merkle_root: Vec<u8>) -> Self {
        Self {
            index_cid,
            ipfs_gateway_addrs,
            merkle_root,
        }
    }
}
