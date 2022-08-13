use digest::Output;

use crate::{digest_ext::DigestExt, node::Node, proof::Proof};

/// A merkle tree generic over some type that can be referenced as bytes.
#[derive(Default, Debug, Clone)]
pub struct MerkleTree<D>
where
    D: DigestExt,
{
    leaves: Vec<Output<D>>,
}

impl<D: DigestExt> MerkleTree<D> {
    pub fn new() -> Self {
        Self {
            leaves: Default::default(),
        }
    }

    pub fn push(&mut self, element: impl AsRef<[u8]>) -> usize {
        let hash: Output<D> = D::new().hash_leaf(element.as_ref());
        self.leaves.push(hash);
        self.leaves.len() - 1
    }

    pub fn push_pre_hash(&mut self, pre_hash: Output<D>) -> usize {
        self.leaves.push(pre_hash);
        self.leaves.len() - 1
    }

    pub fn replace(&mut self, index: usize, element: impl AsRef<[u8]>) {
        if let Some(leaf) = self.leaves.get_mut(index) {
            let mut hash: Output<D> = D::new().hash_leaf(element.as_ref());
            std::mem::swap(leaf, &mut hash);
        }
    }

    pub fn replace_pre_hash(&mut self, index: usize, mut pre_hash: Output<D>) {
        if let Some(leaf) = self.leaves.get_mut(index) {
            std::mem::swap(leaf, &mut pre_hash);
        }
    }

    pub fn root(&self) -> Vec<u8> {
        compute_merkle_root::<D>(&self.leaves).to_vec()
    }

    pub fn generate_proof(&self, index: usize) -> Option<Proof<D>> {
        compute_merkle_proof(&self.leaves, index)
    }
}

impl<D> From<Vec<Output<D>>> for MerkleTree<D>
where
    D: DigestExt,
{
    fn from(leaves: Vec<Output<D>>) -> Self {
        Self { leaves }
    }
}

/// Compute the Merkle root hash for the given slice of `leaves`.
///
/// The values in `leaves` can be any type that implements [`AsRef<[u8]>`][`AsRef`].
///
/// For types implementing [`AsRef<[u8]>`][`AsRef`], the values will be hashed
/// according to the [`Digest`][`DigestExt`] implementation, `D`.
pub fn compute_merkle_root<D>(leaves: &[Output<D>]) -> Output<D>
where
    D: DigestExt,
{
    #[inline]
    fn __generate<D>(digest: &mut D, leaves: &[Output<D>]) -> Output<D>
    where
        D: DigestExt,
    {
        match leaves {
            [] => digest.hash_empty(),
            [leaf] => leaf.clone(),
            leaves => {
                let (this, that): _ = __split_pow2(leaves);

                let lhs: Output<D> = __generate(digest, this);
                let rhs: Output<D> = __generate(digest, that);

                digest.hash_node(&lhs, &rhs)
            }
        }
    }

    __generate::<D>(&mut D::new(), leaves)
}

/// Generate a proof-of-inclusion for the leaf node at the specified `index`.
pub fn compute_merkle_proof<D>(leaves: &[Output<D>], index: usize) -> Option<Proof<D>>
where
    D: DigestExt,
{
    #[inline]
    fn __generate<D>(digest: &mut D, path: &mut Vec<Node<D>>, leaves: &[Output<D>], index: usize)
    where
        D: DigestExt,
    {
        if leaves.len() > 1 {
            let k: usize = __pow2(leaves.len() as u32 - 1);
            let (this, that): _ = leaves.split_at(k);

            if index < k {
                __generate::<D>(digest, path, this, index);
                path.push(Node::R(compute_merkle_root::<D>(that)));
            } else {
                __generate::<D>(digest, path, that, index - k);
                path.push(Node::L(compute_merkle_root::<D>(this)));
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
    use digest::Output;

    use crate::{digest_ext::DigestExt, proof::Proof, MerkleTree};

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

        let mut digest = Blake2b256::new();
        let hashed_leaves: Vec<Output<Blake2b256>> = leaves
            .iter()
            .map(|bytes| digest.hash_leaf(bytes.as_ref()))
            .collect::<Vec<Output<Blake2b256>>>();

        let tree: MerkleTree<Blake2b256> = MerkleTree::from(hashed_leaves.clone());

        let root: Vec<u8> = tree.root();

        let proof: Proof<_> = tree.generate_proof(2).unwrap();

        assert!(!proof.verify_hash(root.as_slice(), hashed_leaves[0]));
        assert!(!proof.verify_hash(root.as_slice(), hashed_leaves[1]));
        assert!(proof.verify_hash(root.as_slice(), hashed_leaves[2]));
        assert!(!proof.verify_hash(root.as_slice(), hashed_leaves[3]));

        assert!(!proof.verify(root.as_slice(), &leaves[0]));
        assert!(!proof.verify(root.as_slice(), &leaves[1]));
        assert!(proof.verify(root.as_slice(), &leaves[2]));
        assert!(!proof.verify(root.as_slice(), &leaves[3]));
    }
}
