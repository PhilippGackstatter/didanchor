// Copyright 2020-2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Formatter;

use digest::Output;

use crate::digest_ext::DigestExt;

/// A tagged hash.
pub enum Node<D: DigestExt> {
    /// A node tagged with `L`.
    L(Output<D>),
    /// A node tagged with `R`.
    R(Output<D>),
}

impl<D: DigestExt> Node<D> {
    /// Computes the parent hash of `self` and `other` using the given `digest`.
    pub fn hash_with(&self, digest: &mut D, other: &Output<D>) -> Output<D> {
        match self {
            Self::L(hash) => digest.hash_node(hash, other),
            Self::R(hash) => digest.hash_node(other, hash),
        }
    }
}

impl<D: DigestExt> std::fmt::Debug for Node<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::L(hash) => f.write_fmt(format_args!("L({:x?})", hash)),
            Self::R(hash) => f.write_fmt(format_args!("R({:x?})", hash)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crypto::hashes::blake2b::Blake2b256;
    use digest::Output;

    use crate::{digest_ext::DigestExt, node::Node};

    #[test]
    fn test_node_hash() {
        let mut digest: Blake2b256 = Blake2b256::new();

        let h1: Output<Blake2b256> = digest.hash_leaf(b"A");
        let h2: Output<Blake2b256> = digest.hash_leaf(b"B");

        assert_eq!(
            Node::L(h1).hash_with(&mut digest, &h2),
            digest.hash_node(&h1, &h2)
        );
        assert_eq!(
            Node::R(h1).hash_with(&mut digest, &h2),
            digest.hash_node(&h2, &h1)
        );

        assert_eq!(
            Node::L(h2).hash_with(&mut digest, &h1),
            digest.hash_node(&h2, &h1)
        );
        assert_eq!(
            Node::R(h2).hash_with(&mut digest, &h1),
            digest.hash_node(&h1, &h2)
        );
    }
}
