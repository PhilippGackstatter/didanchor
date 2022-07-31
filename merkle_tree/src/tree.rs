use digest::Output;

use crate::{digest_ext::DigestExt, node::Node, proof::Proof};

/// A merkle tree generic over some type that can be referenced as bytes.
pub struct MerkleTree<T>
where
    T: AsRef<[u8]>,
{
    leaves: Vec<T>,
}

impl<T> MerkleTree<T>
where
    T: AsRef<[u8]>,
{
    pub fn root<D>(&self) -> Vec<u8>
    where
        D: DigestExt,
    {
        compute_merkle_root::<D, _>(&self.leaves).to_vec()
    }

    pub fn generate_proof<D>(&self, index: usize) -> Option<Proof<D>>
    where
        D: DigestExt,
    {
        compute_merkle_proof(&self.leaves, index)
    }
}

impl<T> From<Vec<T>> for MerkleTree<T>
where
    T: AsRef<[u8]>,
{
    fn from(leaves: Vec<T>) -> Self {
        Self { leaves }
    }
}

/// Compute the Merkle root hash for the given slice of `leaves`.
///
/// The values in `leaves` can be any type that implements [`AsRef<[u8]>`][`AsRef`].
///
/// For types implementing [`AsRef<[u8]>`][`AsRef`], the values will be hashed
/// according to the [`Digest`][`DigestExt`] implementation, `D`.
pub fn compute_merkle_root<D, L>(leaves: &[L]) -> Output<D>
where
    D: DigestExt,
    L: AsRef<[u8]>,
{
    #[inline]
    fn __generate<D, L>(digest: &mut D, leaves: &[L]) -> Output<D>
    where
        D: DigestExt,
        L: AsRef<[u8]>,
    {
        match leaves {
            [] => digest.hash_empty(),
            [leaf] => digest.hash_leaf(leaf.as_ref()),
            leaves => {
                let (this, that): _ = __split_pow2(leaves);

                let lhs: Output<D> = __generate(digest, this);
                let rhs: Output<D> = __generate(digest, that);

                digest.hash_node(&lhs, &rhs)
            }
        }
    }

    __generate::<D, L>(&mut D::new(), leaves)
}

/// Generate a proof-of-inclusion for the leaf node at the specified `index`.
pub fn compute_merkle_proof<D, L>(leaves: &[L], index: usize) -> Option<Proof<D>>
where
    D: DigestExt,
    L: AsRef<[u8]>,
{
    #[inline]
    fn __generate<D, L>(digest: &mut D, path: &mut Vec<Node<D>>, leaves: &[L], index: usize)
    where
        D: DigestExt,
        L: AsRef<[u8]>,
    {
        if leaves.len() > 1 {
            let k: usize = __pow2(leaves.len() as u32 - 1);
            let (this, that): _ = leaves.split_at(k);

            if index < k {
                __generate::<D, L>(digest, path, this, index);
                path.push(Node::R(compute_merkle_root::<D, L>(that)));
            } else {
                __generate::<D, L>(digest, path, that, index - k);
                path.push(Node::L(compute_merkle_root::<D, L>(this)));
            }
        }
    }

    match (index, leaves.len()) {
        (_, 0) => None,
        (0, 1) => Some(Proof::new(Box::new([]))),
        (_, 1) => None,
        (index, length) => {
            if index >= length {
                return None;
            }

            // TODO: Support proofs for any number of leaves
            if !length.is_power_of_two() {
                return None;
            }

            let height: usize = __log2c(leaves.len() as u32) as usize;
            let mut path: Vec<Node<D>> = Vec::with_capacity(height);

            __generate(&mut D::new(), &mut path, leaves, index);

            Some(Proof::new(path.into_boxed_slice()))
        }
    }
}

fn __split_pow2<T>(slice: &[T]) -> (&[T], &[T]) {
    slice.split_at(__pow2(slice.len() as u32 - 1))
}

#[inline]
fn __pow2(value: u32) -> usize {
    1 << __log2c(value)
}

#[inline]
fn __log2c(value: u32) -> u32 {
    32 - value.leading_zeros() - 1
}

#[cfg(test)]
mod tests {
    use crypto::hashes::blake2b::Blake2b256;

    use crate::{proof::Proof, MerkleTree};

    #[derive(Debug, Clone)]
    pub struct TestElement(Vec<u8>);

    impl AsRef<[u8]> for TestElement {
        fn as_ref(&self) -> &[u8] {
            self.0.as_slice()
        }
    }

    fn gen_test_elem() -> TestElement {
        let bytes: [u8; 32] = rand::random();
        TestElement(bytes.to_vec())
    }

    #[test]
    fn test_merkle_tree_proof() {
        let leaves: [TestElement; 4] = [
            gen_test_elem(),
            gen_test_elem(),
            gen_test_elem(),
            gen_test_elem(),
        ];

        let tree: MerkleTree<_> = MerkleTree::from(Vec::from(leaves.clone()));

        let root: Vec<u8> = tree.root::<Blake2b256>();

        let proof: Proof<_> = tree.generate_proof::<Blake2b256>(2).unwrap();

        assert!(!proof.verify(root.as_slice(), &leaves[0]));
        assert!(!proof.verify(root.as_slice(), &leaves[1]));
        assert!(proof.verify(root.as_slice(), &leaves[2]));
        assert!(!proof.verify(root.as_slice(), &leaves[3]));
    }
}
