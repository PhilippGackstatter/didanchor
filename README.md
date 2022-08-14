# DID Anchor

## Running

1. `docker-compose up` to bring up the IPFS cluster.
2. Initialize the necessary config by running `cargo run --example init`.
   - This generates an (insecure!) mnemonic as the seed for private keys. The first address derived from the private keys will be pre-filled with some test funds from the testnet faucet. That is necessary to publish Alias Outputs to the IOTA ledger.
   - It sets sane defaults for the other required configuration parameters, see the generated `anchor_config.json` for their values.
3. `cargo run --example anchor` publishes some test DID documents to the node, which in turn stores them on the IPFS cluster. They are also anchored to the IOTA ledger in an Alias Output.
4. The anchor example prints a DID. We can pass this to the next example: `cargo run --example resolve -- did:iota:...` to resolve it. This will verify two things: The entire Chain of Custody of the DID. It also verifies the merkle proof stored alongside the Chain of Custody, which ensures that the anchoring node has committed to this version of the chain / DID.
