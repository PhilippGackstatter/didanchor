use crypto::hashes::blake2b::Blake2b256;
use identity_core::convert::{FromJson, ToJson};
use merkle_tree::Proof;
use packable::{
    error::{UnpackError, UnpackErrorExt},
    Packable,
};

use crate::ChainOfCustody;

pub struct VerifiableChainOfCustody {
    pub proof: Proof<Blake2b256>,
    pub chain_of_custody: ChainOfCustody,
}

impl VerifiableChainOfCustody {
    pub fn new(proof: Proof<Blake2b256>, chain_of_custody: ChainOfCustody) -> Self {
        Self {
            proof,
            chain_of_custody,
        }
    }
}

impl Packable for VerifiableChainOfCustody {
    type UnpackError = anyhow::Error;

    fn pack<P: packable::packer::Packer>(&self, packer: &mut P) -> Result<(), P::Error> {
        self.proof.pack(packer)?;

        let bytes = self
            .chain_of_custody
            .to_json_vec()
            .expect("TODO: unclear how to use P::Error");

        let len: u64 = bytes.len() as u64;

        len.pack(packer)?;
        packer.pack_bytes(bytes.as_slice())?;

        Ok(())
    }

    fn unpack<U: packable::unpacker::Unpacker, const VERIFY: bool>(
        unpacker: &mut U,
    ) -> Result<Self, packable::error::UnpackError<Self::UnpackError, U::Error>> {
        let proof = <Proof<Blake2b256>>::unpack::<_, VERIFY>(unpacker)?;

        let len: usize = u64::unpack::<_, VERIFY>(unpacker).coerce()? as usize;

        let mut bytes = vec![0; len];
        unpacker.unpack_bytes(&mut bytes)?;
        let chain_of_custody = ChainOfCustody::from_json_slice(&bytes)
            .map_err(|err| UnpackError::Packable(anyhow::anyhow!(err)))?;

        Ok(Self {
            proof,
            chain_of_custody,
        })
    }
}
