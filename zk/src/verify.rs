use crate::prove::{ProofPackage, ProofPackagePrepared};
use ark_bn254::{Bn254, G1Projective};
use ark_ec::pairing::Pairing;
use ark_groth16::{prepare_verifying_key, Groth16, PreparedVerifyingKey, Proof, VerifyingKey};
use ark_serialize::CanonicalDeserialize;

pub fn verify(
    proof: &Proof<Bn254>,
    public_inputs: &G1Projective,
    vk: &VerifyingKey<Bn254>,
) -> bool {
    let pvk = prepare_verifying_key(vk);
    Groth16::<Bn254>::verify_proof_with_prepared_inputs(&pvk, proof, public_inputs).unwrap()
}

pub fn verify_proof_package(
    proof_package: &ProofPackage
) -> bool {
    Groth16::<Bn254>::verify_proof_with_prepared_inputs(&proof_package.prepared_verifying_key, &proof_package.proof, &proof_package.public_inputs).unwrap()
}

pub fn verify_prepared_proof_package(
    proof_package: &ProofPackagePrepared
) -> bool {
    let proof = Proof::<Bn254>::deserialize_uncompressed_unchecked(&proof_package.proof[..]).expect("Error deserializing Proof");
    let prepared_verifying_key = PreparedVerifyingKey::<Bn254>::deserialize_uncompressed_unchecked(&proof_package.verifying_key[..]).expect("Error deserializing PreparedVerifyingKey");
    let projective = G1Projective::deserialize_uncompressed_unchecked(&proof_package.public_inputs[..]).expect("Error deserializing public inputs to Projective");

    let result = Groth16::<Bn254>::verify_proof_with_prepared_inputs(&prepared_verifying_key, &proof, &projective);
    match result {
        Ok(_) => { true }
        Err(_) => { false }
    }
}

