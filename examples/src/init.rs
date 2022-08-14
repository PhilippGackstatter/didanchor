#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use identity_core::crypto::KeyPair;
use identity_core::crypto::KeyType;
use identity_did::did::DID;
use identity_did::verification::MethodRelationship;
use identity_did::verification::MethodScope;

use iota_client::block::address::Address;
use iota_client::block::address::AliasAddress;
use iota_client::block::output::unlock_condition::GovernorAddressUnlockCondition;
use iota_client::block::output::unlock_condition::ImmutableAliasAddressUnlockCondition;
use iota_client::block::output::unlock_condition::StateControllerAddressUnlockCondition;
use iota_client::block::output::AliasId;
use iota_client::block::output::AliasOutput;
use iota_client::block::output::AliasOutputBuilder;
use iota_client::block::output::Output;
use iota_client::block::output::UnlockCondition;
use iota_client::block::Block;
use iota_client::constants::SHIMMER_TESTNET_BECH32_HRP;
use iota_client::crypto::keys::bip39;
use iota_client::node_api::indexer::query_parameters::QueryParameter;
use iota_client::secret::mnemonic::MnemonicSecretManager;
use iota_client::secret::SecretManager;
use iota_client::Client;

static ENDPOINT: &str = "https://api.alphanet.iotaledger.net";
static FAUCET_URL: &str = "https://faucet.alphanet.iotaledger.net/api/enqueue";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client: Client = Client::builder()
        .with_primary_node(ENDPOINT, None)?
        .finish()?;

    get_address_with_funds(&client).await?;

    Ok(())
}

/// Creates a new address and SecretManager with funds from the testnet faucet.
async fn get_address_with_funds(client: &Client) -> anyhow::Result<(Address, SecretManager)> {
    let keypair = identity_core::crypto::KeyPair::new(KeyType::Ed25519)?;
    let mnemonic = iota_client::crypto::keys::bip39::wordlist::encode(
        keypair.private().as_ref(),
        &bip39::wordlist::ENGLISH,
    )
    .map_err(|err| anyhow::anyhow!(format!("{err:?}")))?;

    println!("{mnemonic}");

    let secret_manager =
        SecretManager::Mnemonic(MnemonicSecretManager::try_from_mnemonic(&mnemonic)?);

    let address = client
        .get_addresses(&secret_manager)
        .with_range(0..1)
        .get_raw()
        .await?[0];

    request_faucet_funds(client, address).await?;

    Ok((address, secret_manager))
}

/// Requests funds from the testnet faucet for the given `address`.
async fn request_faucet_funds(client: &Client, address: Address) -> anyhow::Result<()> {
    let address_bech32 = address.to_bech32(SHIMMER_TESTNET_BECH32_HRP);

    iota_client::request_funds_from_faucet(FAUCET_URL, &address_bech32).await?;

    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            let balance = get_address_balance(client, &address_bech32).await?;
            if balance > 0 {
                break;
            }
        }
        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(())
}

/// Returns the balance of the given bech32-encoded `address`.
async fn get_address_balance(client: &Client, address: &str) -> anyhow::Result<u64> {
    let output_ids = client
        .basic_output_ids(vec![
            QueryParameter::Address(address.to_owned()),
            QueryParameter::HasExpiration(false),
            QueryParameter::HasTimelock(false),
            QueryParameter::HasStorageDepositReturn(false),
        ])
        .await?;

    let outputs_responses = client.get_outputs(output_ids).await?;

    let mut total_amount = 0;
    for output_response in outputs_responses {
        let output = Output::try_from(&output_response.output)?;
        total_amount += output.amount();
    }

    Ok(total_amount)
}