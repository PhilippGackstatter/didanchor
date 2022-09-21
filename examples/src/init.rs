use didanchor::AnchorConfig;
use didanchor::IpfsNodeManagementAddress;
use didanchor::IpfsNodePublicAddress;
use identity_core::crypto::KeyType;
use iota_client::block::address::Address;
use iota_client::block::output::AliasId;
use iota_client::block::output::Output;
use iota_client::constants::SHIMMER_TESTNET_BECH32_HRP;
use iota_client::crypto::keys::bip39;
use iota_client::node_api::indexer::query_parameters::QueryParameter;
use iota_client::secret::mnemonic::MnemonicSecretManager;
use iota_client::secret::SecretManager;
use iota_client::Client;
use multiaddr::multihash::Multihash;
use multiaddr::{Multiaddr, Protocol};
use url::Url;

static DEFAULT_ENDPOINT: &str = "https://api.testnet.shimmer.network/";
static FAUCET_URL: &str = "https://faucet.testnet.shimmer.network/api/enqueue";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client: Client = Client::builder()
        .with_primary_node(DEFAULT_ENDPOINT, None)?
        .finish()?;

    let (mnemonic, _, _) = get_address_with_funds(&client).await?;

    // Ports match docker-compose.yml.
    let swarm_port: u16 = 4002;
    let api_port: u16 = 5002;
    let gateway_port: u16 = 8081;
    let cluster_port: u16 = 9094;

    let mut pub_node_urls: Vec<IpfsNodePublicAddress> = Vec::new();
    let mut mgmt_node_urls: Vec<IpfsNodeManagementAddress> = Vec::new();

    // Get the peer ids from the nodes so we don't have to hardcode them.
    for i in 0..=2u16 {
        let node_addr = format!("/ip4/127.0.0.1/tcp/{}", api_port + i).parse::<Multiaddr>()?;

        let mut addr_iter = node_addr.iter();
        let ip_addr = if let Some(Protocol::Ip4(addr)) = addr_iter.next() {
            addr
        } else {
            anyhow::bail!("expected ip4 protocol");
        };

        let port = if let Some(Protocol::Tcp(port)) = addr_iter.next() {
            port
        } else {
            anyhow::bail!("expected tcp protocol");
        };

        let node_url = Url::parse(&format!(
            "http://{}.{}.{}.{}:{port}",
            ip_addr.octets()[0],
            ip_addr.octets()[1],
            ip_addr.octets()[2],
            ip_addr.octets()[3]
        ))
        .unwrap();

        let ipfs_client = ipfs_client::IpfsClient::new(vec![node_url]).unwrap();
        let config = ipfs_client.config_show().await?;
        let peer_id: &str = config
            .get("Identity")
            .unwrap()
            .get("PeerID")
            .unwrap()
            .as_str()
            .unwrap();

        let peer_id_bytes: Vec<u8> = bs58::decode(peer_id).into_vec().unwrap();
        let peer_id_multihash: Multihash = Multihash::from_bytes(&peer_id_bytes).unwrap();
        let peer_id_protocol: Protocol = Protocol::P2p(peer_id_multihash);

        pub_node_urls.push(IpfsNodePublicAddress {
            host: Protocol::Ip4(ip_addr),
            swarm_port: Multiaddr::from_iter([Protocol::Udp(swarm_port + i), Protocol::Quic]),
            gateway_port: Protocol::Tcp(gateway_port + i),
            peer_id: peer_id_protocol,
        });

        mgmt_node_urls.push(IpfsNodeManagementAddress {
            host: ip_addr.to_string(),
            api_port: api_port + i,
            cluster_port: cluster_port + i,
        });
    }

    let config = AnchorConfig {
        alias_id: AliasId::null(),
        mnemonic,
        iota_endpoint: DEFAULT_ENDPOINT.to_owned(),
        ipfs_node_public_addrs: pub_node_urls,
        ipfs_node_management_addrs: mgmt_node_urls,
    };

    config.write_default_location().await?;

    println!("initialized {}", AnchorConfig::DEFAULT_PATH);

    Ok(())
}

/// Creates a new address and SecretManager with funds from the testnet faucet.
async fn get_address_with_funds(
    client: &Client,
) -> anyhow::Result<(String, Address, SecretManager)> {
    let keypair = identity_core::crypto::KeyPair::new(KeyType::Ed25519)?;
    let mnemonic = iota_client::crypto::keys::bip39::wordlist::encode(
        keypair.private().as_ref(),
        &bip39::wordlist::ENGLISH,
    )
    .map_err(|err| anyhow::anyhow!(format!("{err:?}")))?;

    let secret_manager =
        SecretManager::Mnemonic(MnemonicSecretManager::try_from_mnemonic(&mnemonic)?);

    let address = client
        .get_addresses(&secret_manager)
        .with_range(0..1)
        .get_raw()
        .await?[0];

    request_faucet_funds(client, address).await?;

    Ok((mnemonic, address, secret_manager))
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
