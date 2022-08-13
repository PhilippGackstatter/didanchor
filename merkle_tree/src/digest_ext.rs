// Copyright 2020-2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[doc(inline)]
pub use crypto::hashes::Digest;
#[doc(inline)]
pub use crypto::hashes::Output;
use digest::FixedOutputReset;
use digest::Reset;
use typenum::Unsigned;

/// Leaf domain separation prefix.
const PREFIX_LEAF: &[u8] = &[0x00];

/// Node domain separation prefix.
const PREFIX_NODE: &[u8] = &[0x01];

/// An extension of the [`Digest`] trait for Merkle tree construction.
pub trait DigestExt: Sized + Digest + Reset + FixedOutputReset {
    /// The output size of the digest function.
    const OUTPUT_SIZE: usize;

    /// Computes the [`struct@Hash`] of a Merkle tree leaf node.
    fn hash_leaf(&mut self, data: &[u8]) -> Output<Self> {
        Digest::reset(self);
        Digest::update(self, PREFIX_LEAF);
        Digest::update(self, data);
        self.finalize_reset()
    }

    /// Computes the parent [`struct@Hash`] of two Merkle tree nodes.
    fn hash_node(&mut self, lhs: &Output<Self>, rhs: &Output<Self>) -> Output<Self> {
        Digest::reset(self);
        Digest::update(self, PREFIX_NODE);
        Digest::update(self, lhs.as_slice());
        Digest::update(self, rhs.as_slice());
        self.finalize_reset()
    }

    /// Computes the [`struct@Hash`] of an empty Merkle tree.
    fn hash_empty(&mut self) -> Output<Self> {
        Digest::reset(self);
        Digest::update(self, &[]);
        self.finalize_reset()
    }
}

impl<D> DigestExt for D
where
    D: Digest + Reset + FixedOutputReset,
{
    const OUTPUT_SIZE: usize = <D::OutputSize>::USIZE;
}
