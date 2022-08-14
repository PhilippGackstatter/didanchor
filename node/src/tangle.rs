use identity_core::convert::ToJson;
use iota_client::{
    api_types::responses::OutputResponse,
    block::{
        address::Address,
        output::{
            unlock_condition::{
                GovernorAddressUnlockCondition, StateControllerAddressUnlockCondition,
            },
            AliasId, AliasOutput, AliasOutputBuilder, Output, OutputId, RentStructure,
            UnlockCondition,
        },
        Block,
    },
    secret::{mnemonic::MnemonicSecretManager, SecretManager},
    Client,
};

static ENDPOINT: &str = "https://api.alphanet.iotaledger.net";
// static FAUCET_URL: &str = "https://faucet.alphanet.iotaledger.net/api/enqueue";

pub struct AnchorAlias {
    client: Client,
    id: Option<AliasId>,
    // insecure.
    secret_manager: SecretManager,
}

impl AnchorAlias {
    pub fn new(mnemonic: String) -> anyhow::Result<Self> {
        let client: Client = Client::builder()
            .with_primary_node(ENDPOINT, None)?
            .finish()?;

        let secret_manager: SecretManager =
            SecretManager::Mnemonic(MnemonicSecretManager::try_from_mnemonic(&mnemonic)?);

        Ok(AnchorAlias {
            client,
            id: None,
            secret_manager,
        })
    }

    pub fn new_with_id(id: AliasId, mnemonic: String) -> anyhow::Result<Self> {
        let mut anchor_alias = Self::new(mnemonic)?;
        anchor_alias.id = Some(id);
        Ok(anchor_alias)
    }

    pub async fn publish_output(&self, content: AliasContent) -> anyhow::Result<()> {
        let content_vec = content.to_json_vec()?;

        let rent_structure = self.client.get_rent_structure().await?;

        let alias_output: AliasOutput = match self.id {
            Some(alias_id) => {
                self.update_output(content_vec, rent_structure, alias_id)
                    .await?
            }
            None => self.new_output(content_vec, rent_structure).await?,
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

        Ok(())
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
        let (alias_id, _, alias_output) = self.resolve_alias_output(alias_id).await?;

        let mut alias_output_builder: AliasOutputBuilder = AliasOutputBuilder::from(&alias_output)
            .with_minimum_storage_deposit(rent_structure)
            .with_state_index(alias_output.state_index() + 1)
            .with_state_metadata(state_metadata);

        if alias_output.alias_id().is_null() {
            alias_output_builder = alias_output_builder.with_alias_id(alias_id);
        }

        Ok(alias_output_builder.finish()?)
    }

    /// Resolve a did into an Alias Output and the associated identifiers.
    async fn resolve_alias_output(
        &self,
        alias_id: AliasId,
    ) -> anyhow::Result<(AliasId, OutputId, AliasOutput)> {
        let output_id: OutputId = self.client.alias_output_id(alias_id).await?;
        let output_response: OutputResponse = self.client.get_output(&output_id).await?;
        let output: Output = Output::try_from(&output_response.output)?;

        if let Output::Alias(alias_output) = output {
            Ok((alias_id, output_id, alias_output))
        } else {
            unreachable!("we requested an alias output. (TODO: turn into error later, though.)");
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AliasContent {
    index_cid: String,
    node_addrs: Vec<String>,
    merkle_root: Vec<u8>,
}
