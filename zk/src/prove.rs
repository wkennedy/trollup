use crate::account_state_circuit::AccountStateCircuit;
use crate::byte_utils::{bytes_to_field, fr_to_g1, g1_affine_to_bytes};
use ark_bn254::{Bn254, Fr, G1Projective};
use ark_groth16::{prepare_verifying_key, Groth16, PreparedVerifyingKey, Proof, ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, CanonicalSerializeHashExt, Compress};
use ark_snark::SNARK;
use borsh::{BorshDeserialize, BorshSerialize};
use rand::thread_rng;
use std::fs::File;
use std::io::{Read, Write};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use state::account_state::AccountState;

//TODO we know the size of the proof and vk, so change from vec
#[derive(BorshSerialize, BorshDeserialize)]
pub struct ProofPackageLite {
    pub proof: Vec<u8>,
    pub public_inputs: Vec<[u8; 32]>,
    pub verifying_key: Vec<u8>
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct ProofPackagePrepared {
    pub proof: Vec<u8>,
    pub public_inputs: Vec<u8>,
    pub verifying_key: Vec<u8>
}

impl Into<ProofPackage> for ProofPackagePrepared {
    fn into(self) -> ProofPackage {
        let proof = Proof::<Bn254>::deserialize_uncompressed_unchecked(&self.proof[..]).expect("Error deserializing Proof");
        let prepared_verifying_key = PreparedVerifyingKey::<Bn254>::deserialize_uncompressed_unchecked(&self.verifying_key[..]).expect("Error deserializing PreparedVerifyingKey");
        let projective = G1Projective::deserialize_uncompressed_unchecked(&self.public_inputs[..]).expect("Error deserializing public inputs to Projective");
        ProofPackage {
            proof,
            public_inputs: projective,
            prepared_verifying_key,
        }
    }
}

pub struct ProofPackage {
    pub proof: Proof<Bn254>,
    pub public_inputs: G1Projective,
    pub prepared_verifying_key: PreparedVerifyingKey<Bn254>
}

// TODO Handle generation of proving key elsewhere and load it differently
pub fn setup(save_keys: bool) -> (ProvingKey<Bn254>, VerifyingKey<Bn254>){
    let rng = &mut thread_rng();
    let account_state_circuit = AccountStateCircuit::default();
    let (proving_key, verifying_key) = Groth16::<Bn254>::circuit_specific_setup(account_state_circuit.clone(), rng).unwrap();

    if save_keys {
        let mut pk_file = File::create("pk.bin").unwrap();
        let mut pk_bytes = Vec::new();
        proving_key.serialize_uncompressed(&mut pk_bytes).expect("");
        pk_file.write(&pk_bytes).expect("TODO: panic message");

        let mut file = File::create("vk.bin").unwrap();
        let mut vk_bytes = Vec::new();
        verifying_key.serialize_uncompressed(&mut vk_bytes).expect("");
        file.write(&vk_bytes).expect("TODO: panic message");
    };

    (proving_key, verifying_key)
}

pub fn generate_proof_load_keys(accounts: &Vec<AccountState>) -> (ProofPackageLite, ProofPackagePrepared, ProofPackage) {
    // Open the file
    let mut pk_file = File::open("pk.bin").expect("");

    // Read the contents of the file
    let mut pk_buffer = Vec::new();
    pk_file.read_to_end(&mut pk_buffer).expect("");

    // Deserialize the buffer into a VerifyingKey
    let pk = ProvingKey::<Bn254>::deserialize_uncompressed_unchecked(&pk_buffer[..]).expect("");

    // Open the file
    let mut vk_file = File::open("vk.bin").expect("");

    // Read the contents of the file
    let mut vk_buffer = Vec::new();
    vk_file.read_to_end(&mut vk_buffer).expect("");

    // Deserialize the buffer into a VerifyingKey
    let vk = VerifyingKey::<Bn254>::deserialize_uncompressed_unchecked(&vk_buffer[..]).expect("");

    generate_proof(&pk, &vk, accounts)
}

pub fn generate_proof(proving_key: &ProvingKey<Bn254>, verifying_key: &VerifyingKey<Bn254>, accounts: &Vec<AccountState>) -> (ProofPackageLite, ProofPackagePrepared, ProofPackage) {
    let rng = &mut thread_rng();

    let account_state_circuit = AccountStateCircuit::new(accounts.clone());
    let public_inputs = account_state_circuit.public_inputs();

    let proof = Groth16::<Bn254>::prove(&proving_key,
                                        account_state_circuit,
                                        rng,
    ).unwrap();

    let mut proof_bytes = Vec::with_capacity(proof.serialized_size(Compress::No));
    proof.serialize_uncompressed(&mut proof_bytes).expect("Error serializing proof");

    let public_inputs_fr = public_inputs
        .iter()
        .map(|input| bytes_to_field(input))
        .collect::<Result<Vec<Fr>, _>>().expect("");

    let prepared_verifying_key = prepare_verifying_key(&verifying_key);

    let g1_projective: G1Projective = Groth16::<Bn254>::prepare_inputs(&prepared_verifying_key, &public_inputs_fr).expect("Error preparing inputs with public inputs and prepared verifying key");

    let mut projective_bytes: Vec<u8> = Vec::new();
    let _ = g1_projective.serialize_uncompressed(&mut projective_bytes);
    let mut verifying_key_bytes: Vec<u8> = Vec::with_capacity(verifying_key.serialized_size(Compress::No));
    let _ = verifying_key.serialize_uncompressed(&mut verifying_key_bytes);
    let mut prepared_verifying_key_bytes: Vec<u8> = Vec::new();
    let _ = prepared_verifying_key.serialize_uncompressed(&mut prepared_verifying_key_bytes);

    (ProofPackageLite {
        proof: proof_bytes.clone(),
        public_inputs: public_inputs.clone(),
        verifying_key: prepared_verifying_key_bytes.clone(),
    },
     ProofPackagePrepared {
        proof: proof_bytes,
        public_inputs: projective_bytes,
        verifying_key: prepared_verifying_key_bytes,
    },
     ProofPackage {
        proof,
        public_inputs: g1_projective,
        prepared_verifying_key,
    })
}
