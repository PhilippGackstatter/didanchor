// Copyright 2020-2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Formatter;

use digest::Output;
use packable::error::UnpackErrorExt;
use packable::Packable;

use crate::digest_ext::DigestExt;
use crate::node::Node;

/// Maximum number of nodes in the proof.
/// This value is equal to log₂MAX_KEYS_ALLOWED, respecting the constraint for the maximum number of keys allowed in a
/// `KeyCollection`
// pub const MAX_PROOF_NODES: usize = 12;

/// An Merkle tree inclusion proof that allows proving the existence of a
/// particular leaf in a Merkle tree.
pub struct Proof<D: DigestExt> {
    nodes: Box<[Node<D>]>,
}

impl<D: DigestExt> Proof<D> {
    /// Creates a new [`Proof`] from a boxed slice of nodes.
    pub fn new(nodes: Box<[Node<D>]>) -> Self {
        // if nodes.len() > MAX_PROOF_NODES {
        //   return Err(Error::InvalidProofSize(nodes.len()));
        // }
        Self { nodes }
    }

    /// Returns the nodes as a slice.
    pub fn nodes(&self) -> &[Node<D>] {
        &self.nodes
    }

    /// Returns the index of underlying leaf node in the Merkle tree.
    pub fn index(&self) -> usize {
        self.nodes
            .iter()
            .enumerate()
            .fold(0, |acc, (depth, node)| match node {
                Node::L(_) => acc + 2_usize.pow(depth as u32),
                Node::R(_) => acc,
            })
    }

    /// Verifies the computed root of `self` with the given `root` hash.
    pub fn verify<T>(&self, root: &[u8], target: T) -> bool
    where
        T: AsRef<[u8]>,
    {
        self.verify_hash(root, D::new().hash_leaf(target.as_ref()))
    }

    /// Verifies the computed root of `self` with the given `root` hash and
    /// a pre-computed target `hash`.
    pub fn verify_hash(&self, root: &[u8], hash: Output<D>) -> bool {
        self.root(hash).as_slice() == root
    }

    /// Computes the root hash from `target` using a default digest.
    pub fn root(&self, target: Output<D>) -> Output<D> {
        self.root_with(&mut D::new(), target)
    }

    /// Computes the root hash from `target` using the given `digest`.
    pub fn root_with(&self, digest: &mut D, target: Output<D>) -> Output<D> {
        self.nodes
            .iter()
            .fold(target, |acc, item| item.hash_with(digest, &acc))
    }
}

impl<D: DigestExt + 'static> Packable for Proof<D> {
    type UnpackError = anyhow::Error;

    fn pack<P: packable::packer::Packer>(&self, packer: &mut P) -> Result<(), P::Error> {
        let len: u64 = self.nodes.len() as u64;
        len.pack(packer)?;

        for node in self.nodes.iter() {
            node.pack(packer)?;
        }

        Ok(())
    }

    fn unpack<U: packable::unpacker::Unpacker, const VERIFY: bool>(
        unpacker: &mut U,
    ) -> Result<Self, packable::error::UnpackError<Self::UnpackError, U::Error>> {
        let len: u64 = u64::unpack::<_, VERIFY>(unpacker).coerce()?;
        let mut nodes: Vec<Node<D>> = Vec::with_capacity(len as usize);

        for _ in 0..len {
            nodes.push(<Node<D>>::unpack::<_, VERIFY>(unpacker)?);
        }

        Ok(Proof::new(nodes.into_boxed_slice()))
    }
}

// impl<D: DigestExt> Clone for Proof<D>
// where
//     Node<D>: Clone,
// {
//     fn clone(&self) -> Self {
//         Self {
//             nodes: self.nodes.clone(),
//         }
//     }
// }

impl<D: DigestExt> std::fmt::Debug for Proof<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Proof").field("nodes", &self.nodes).finish()
    }
}
