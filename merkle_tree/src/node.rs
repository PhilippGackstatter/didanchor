// Copyright 2020-2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Formatter;

use digest::Output;
use packable::{
    error::{UnpackError, UnpackErrorExt},
    packer::Packer,
    unpacker::Unpacker,
    Packable,
};

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

impl<D: DigestExt + 'static> Packable for Node<D> {
    type UnpackError = anyhow::Error;

    fn pack<P: Packer>(&self, packer: &mut P) -> Result<(), P::Error> {
        match self {
            Node::L(left) => {
                0u8.pack(packer)?;
                packer.pack_bytes(left.as_slice())
            }
            Node::R(right) => {
                1u8.pack(packer)?;
                packer.pack_bytes(right.as_slice())
            }
        }
    }

    fn unpack<U: Unpacker, const VERIFY: bool>(
        unpacker: &mut U,
    ) -> Result<Self, UnpackError<Self::UnpackError, U::Error>> {
        let tag: u8 = u8::unpack::<_, VERIFY>(unpacker).coerce()?;

        if tag > 1 {
            return Err(UnpackError::Packable(anyhow::anyhow!(tag)));
        }

        let len: usize = unpacker.read_bytes().ok_or_else(|| {
            UnpackError::Packable(anyhow::anyhow!(
                "TODO: When does this return None, exactly?"
            ))
        })?;

        if len != D::OUTPUT_SIZE {
            return Err(UnpackError::Packable(anyhow::anyhow!("unexpected size")));
        }

        let mut bytes: Vec<u8> = Vec::with_capacity(len);
        unpacker.unpack_bytes(&mut bytes)?;

        let output: Output<D> = Output::<D>::from_exact_iter(bytes.into_iter())
            .expect("the size should be correct as we just checked");

        if tag == 0u8 {
            Ok(Node::L(output))
        } else {
            Ok(Node::R(output))
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
