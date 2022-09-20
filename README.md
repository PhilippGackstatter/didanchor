# DID Anchor

## How it works

A library for anchoring DID Documents in the IOTA ledger.

The `didanchor` library allows publishing DID documents on the IPFS network by hosting an IPFS cluster where the documents are mirrored across all of the cluster's nodes. The library maintains an index from [DIDs (decentralized identifiers)](https://www.w3.org/TR/did-core/) to [CIDs (content identifiers)](https://docs.ipfs.tech/concepts/glossary/#cid). When a CID is resolved resolved on IPFS, it yields a _chain of custody_. The chain of custody is essentially a list of DID documents. The chain allows updating a DID document over time, essentially by adding patches to the chain. Any observer can verify the chain by applying the digitally signed patches to the previous document of the chain. This allows anyone to verify the latest state without having to trust a central authority. The IPFS network's responsibility in this setup becomes only that of data availability.

What's left is the need to obtain the index in a timely manner. Since the index needs a stable location from where it can be fetched and IPNS is very slow, an alternative storage mechanism is required. The library publishes (or _anchors_) the index into the IOTA network, a distributed ledger. It uses an Alias Output in the IOTA ledger which is associated with a globally unique identifier.

DIDs published with this have the following schema:

```
did:iota:<alias_id>:<did_tag>
```

A DID resolver obtains the `alias_id` from the DID and resolves the corresponding Alias Output on the IOTA ledger. From there it extracts the CID of the index and resolves it on the IPFS network. Next to the index CID, the Alias Output contains the network addresses of the IPFS cluster nodes so they can be peered with directly, which significantly speeds up resolution. Next they map the `did_tag` to its current CID using the index, and subsequently resolving that CID on IPFS. That yields a chain of custody which can be processed into a DID document.

Note that the usage of `did:iota` is something to be fixed. This library does not implement the IOTA DID method specification.

### Proof of Inclusion

In order to prove that a certain DID document existed in an earlier version of an Alias Output, an observer could generate a proof of inclusion of the Alias Output in the IOTA ledger, keep a copy of the index (which is cryptographically identified by its CID) and a copy of the chain of custody (which is also identified by its CID). All those parts allow an observer to cryptographically convince someone else that a certain DID document existed. This is useful if, say, the owner of a DID digitally signed a mortgage with a key but rotated that key out since. The bank could keep these aforementioned parts (constituting a proof of inclusion) to convince someone (like a court) that the DID owner did indeed sign the mortgage at some point.

A slightly more efficient method of doing this is to avoid the copy of the index, which might become significant in size. To that end, each chain of custody also comes with a merkle proof. Together with the merkle root contained in the Alias Output, it allows an observer to verify that a certain chain of custody, and thus a certain state of a DID document, was indeed published by the controller of the Alias Output. This is simply a more efficient way to prove the existence of a certain state than keeping a copy of the index. For a large publisher, the merkle proof will be orders of magnitude smaller than the index. Copies of the Alias Output and Chain of Custody are still required, however.

With all that said, the usefulness of this proof of inclusion and the additional complexity it introduces is debatable, and it may or may not be removed in the future.

## Resolution

During resolution there are multiple ways to obtain the bytes to a given CID. There are at least three ways to do it, which have varying trust considerations.

1. Setup an IPFS cluster that subscribes to the cluster used by the DID publisher one is interested in. It would mirror all their DIDs on the local cluster, in which case the local cluster can be used to lookup a chain of custody. This is not only a highly trustworthy setup, since the cluster on which the CIDs are looked up is self-hosted, it also increases the availability of the publisher's DIDs. However, it is also the most technically involved and expensive.
2. A lighter-weight alternative is to run just a single IPFS node locally and instruct it to peer with any or all of the publisher's cluster nodes. Then the lookup can be done via the local IPFS node. The local IPFS node will verify that the IPFS blocks returned from the cluster together match the requested CID. This is what the library currently uses when resolving.
3. Another alternative is to use the IPFS HTTP gateway of the publisher's cluster nodes. This requires the least setup of all options, but requires additional validation to ensure that what the gateway returns matches the requested CID so the resolver doesn't have to trust the HTTP gateway but can do "trustless resolution". This is a technically solvable issue, but not currently implemented.

## Running

1. `docker-compose up` to bring up the IPFS cluster.
2. Initialize the necessary config by running `cargo run --example init`.
   - This generates a mnemonic as the seed for private keys. The first address derived from the private keys will be pre-filled with some test funds from the testnet faucet. That is necessary to publish Alias Outputs to the IOTA ledger.
   - It sets defaults for the other required configuration parameters, such as the IOTA network to use. See the generated `anchor_config.toml` for their values.
3. `cargo run --example anchor` adds 4 test DID documents to the DID Anchor, which holds it in memory until committed. When the changes are committed, the DID documents are published to the IPFS cluster. Subsequently, they are anchored to the IOTA ledger in an Alias Output.
4. The anchor example prints multiple DIDs that were published. We can pass any of those to the next example: `cargo run --example resolve -- did:iota:...` to resolve it. Note that this requires a running local ipfs daemon, which can be run with `ipfs daemon`. This will verify two things: The entire chain of custody of the DID and the merkle proof stored alongside the chain of custody, which ensures that the anchoring node has indeed committed to this version of the DID document.

## State of the library

The library is in a proof-of-concept state and not ready for production use. A non-exhaustive list of outstanding tasks to get to a production ready state is:

- Proper error handling, including not using `anyhow` and `unwrap` all the time.
- DIDs in documents only contain their tag, but not the Alias Id of the publisher.
- The merkle tree implementation only supports powers of 2 as the number of leaves. If the proof of inclusion is to be kept, it should perhaps be replaced by a verkle tree for more efficiency.
- Testing things, particularly non-happy paths.

Eventually, a node implementation that exposes the library's API via HTTP is desirable, too.
