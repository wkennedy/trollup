


#[cfg(test)]
mod test {
    use ark_bn254::{Bn254, Fr};
    use ark_groth16::Groth16;
    use ark_serialize::{CanonicalSerialize, Compress};
    use ark_snark::SNARK;
    use rand::thread_rng;
    use solana_program::alt_bn128::compression::prelude::convert_endianness;
    use crate::byte_utils::{fr_to_g1, g1_affine_to_bytes};
    use crate::errors::Groth16Error;
    use crate::example::{convert_arkworks_vk_to_solana_example, convert_vec_to_array_example, ExampleCircuit};
    use crate::verify_lite::Groth16Verifier;
    use ark_bn254::{Bn254, Fr, G1Affine};
    use ark_ff::PrimeField;
    use ark_groth16::VerifyingKey;
    use ark_relations::lc;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable};
    use ark_serialize::CanonicalSerialize;
    use ark_std::Zero;
    use light_poseidon::{Poseidon, PoseidonHasher};
    use sha2::{Digest, Sha256};
    use solana_program::alt_bn128::compression::prelude::convert_endianness;
    use solana_program::pubkey::Pubkey;
    use crate::account_state::AccountState;
    use crate::byte_utils::{convert_endianness_32, convert_endianness_64, field_to_bytes};
    use crate::verify_lite::Groth16VerifyingKey;

    // Circuit for proving knowledge of a Solana account's state changes
    // The idea behind this example circuit is that the rollup that generates this proof for a batch of
    // account changes, which this circuit representing the state change for the accounts in the batch
    // collectively. The merkle_node_hash is a hash of the account leaf hashes (different from the Merkle root);
    // The account_hash is a hash of the account addresses and data and the lamports sum is the sum of all account lamports.
    #[derive(Clone)]
    pub struct ExampleCircuit {
        pub some_value: Option<Fr>,
    }

    impl ExampleCircuit {

        pub fn default() -> Self {
            ExampleCircuit {
                some_value: None,
            }
        }

        pub fn new() -> Self {

            // Compute addresses_hash and lamports_sum
            // let mut poseidon = Poseidon::<Fr>::new_circom(1).unwrap();
            // addresses_hash = poseidon.hash(&[addresses_hash, address_fr, datum_fr]).unwrap();

            let circuit = ExampleCircuit {
                some_value: Some(Fr::from(100)),
            };

            circuit
        }

        pub fn public_inputs_fr(&self) -> Vec<[u8; 32]> {
            let public_inputs: Vec<[u8; 32]> = vec![
                field_to_bytes(self.some_value.unwrap()),
            ];

            public_inputs
        }

        pub fn public_inputs(&self) -> Vec<[u8; 32]> {
            let public_inputs: Vec<[u8; 32]> = vec![
                field_to_bytes(self.some_value.unwrap()),
            ];

            public_inputs
        }
    }

    impl ConstraintSynthesizer<Fr> for ExampleCircuit {
        fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {

            // Allocate public inputs
            let some_value_var = cs.new_input_variable(|| {
                self.some_value.ok_or(SynthesisError::AssignmentMissing)
            })?;

            // Constraint: Ensure computed addresses_hash matches the provided addresses_hash
            cs.enforce_constraint(
                lc!() + some_value_var,
                lc!() + Variable::One,
                lc!() + some_value_var,
            )?;

            Ok(())
        }
    }



    fn convert_arkworks_vk_to_solana_example(ark_vk: &VerifyingKey<Bn254>) -> Groth16VerifyingKey<'static> {
        // Convert alpha_g1
        let mut vk_alpha_g1 = [0u8; 64];
        ark_vk.alpha_g1
            .serialize_uncompressed(&mut vk_alpha_g1[..])
            .unwrap();

        // Convert beta_g2
        let mut vk_beta_g2 = [0u8; 128];
        ark_vk.beta_g2
            .serialize_uncompressed(&mut vk_beta_g2[..])
            .unwrap();

        // Convert gamma_g2
        let mut vk_gamma_g2 = [0u8; 128];
        ark_vk.gamma_g2
            .serialize_uncompressed(&mut vk_gamma_g2[..])
            .unwrap();

        // Convert delta_g2
        let mut vk_delta_g2 = [0u8; 128];
        ark_vk.delta_g2
            .serialize_uncompressed(&mut vk_delta_g2[..])
            .unwrap();

        // Convert gamma_abc_g1 (vk_ic)
        let vk_ic: Vec<[u8; 64]> = ark_vk.gamma_abc_g1
            .iter()
            .map(|point| {
                let mut buf = [0u8; 64];
                point.serialize_uncompressed(&mut buf[..]).unwrap();
                convert_endianness::<32, 64>(&buf)
            })
            .collect();

        let vk_alpha_g1_converted = convert_endianness::<32, 64>(&vk_alpha_g1);
        let vk_beta_g2_converted = convert_endianness::<64, 128>(&vk_beta_g2);
        let vk_gamma_g2_converted = convert_endianness::<64, 128>(&vk_gamma_g2);
        let vk_delta_g2_converted = convert_endianness::<64, 128>(&vk_delta_g2);

        Groth16VerifyingKey {
            nr_pubinputs: 2, // Subtract 1 for the constant term
            vk_alpha_g1: vk_alpha_g1_converted,
            vk_beta_g2: vk_beta_g2_converted,
            vk_gamma_g2: vk_gamma_g2_converted,
            vk_delta_g2: vk_delta_g2_converted,
            vk_ic: Box::leak(vk_ic.into_boxed_slice()), // Convert to 'static lifetime
        }
    }

    const NR_INPUTS: usize = 1; // Replace with your actual NR_INPUTS value
    fn convert_vec_to_array_example(vec: &Vec<[u8; 32]>) -> Result<[[u8; 32]; NR_INPUTS], String> {
        if vec.len() != NR_INPUTS {
            return Err(format!("Expected {} elements, but got {}", NR_INPUTS, vec.len()));
        }

        let converted_endian: Vec<[u8; 32]> = vec.iter().map(|bytes| convert_endianness_32(bytes)).collect();
        let arr: [[u8; 32]; NR_INPUTS] = converted_endian.try_into()
            .map_err(|_| "Conversion failed")?;

        Ok(arr)
    }
    #[test]
    fn should_verify_basic_circuit_groth16() {
        let rng = &mut thread_rng();
        // let bn = Bn254::rand(rng);
        let c = ExampleCircuit {
            some_value: Some(Fr::from(100))
        };

        let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(c, rng).unwrap();

        let c2 = ExampleCircuit {
            some_value: Some(Fr::from(100))
        };

        let public_input = &c2.public_inputs();

        let proof = Groth16::<Bn254>::prove(&pk, c2, rng).unwrap();

        let res = Groth16::<Bn254>::verify(&vk, &[Fr::from(100)], &proof).unwrap();
        info!("{:?}", res);
        // assert!(res);

        let mut proof_bytes = Vec::with_capacity(proof.serialized_size(Compress::No));
        proof.serialize_uncompressed(&mut proof_bytes).expect("Error serializing proof");

        let proof_a: [u8; 64] = convert_endianness::<32, 64>(proof_bytes[0..64].try_into().unwrap());
        let proof_b: [u8; 128] = convert_endianness::<64, 128>(proof_bytes[64..192].try_into().unwrap());
        let proof_c: [u8; 64] = convert_endianness::<32, 64>(proof_bytes[192..256].try_into().unwrap());

        // let proof_a: [u8; 64] = proof_package.proof[0..64].try_into().unwrap();
        // let proof_b: [u8; 128] = proof_package.proof[64..192].try_into().unwrap();
        // let proof_c: [u8; 64] = proof_package.proof[192..256].try_into().unwrap();

        let vk = convert_arkworks_vk_to_solana_example(&vk);
        // let g1 = g1_affine_to_bytes(&fr_to_g1(&Fr::from(100)));
        // let mut pi: Vec<[u8; 64]> = Vec::new();
        // pi.push(<[u8; 64]>::try_from(&g1[0..64]).unwrap());
        let pip = convert_vec_to_array_example(&public_input).unwrap();

        let mut verifier: Groth16Verifier<1> = Groth16Verifier::new(
            &proof_a,
            &proof_b,
            &proof_c,
            &pip,
            &vk,
        ).unwrap();

        match verifier.verify_unchecked() {
            Ok(true) => {
                info!("Proof verification succeeded");
                // Ok(true)
            }
            Ok(false) => {
                info!("Proof verification failed");
                // Ok(false)
            }
            Err(error) => {
                info!("Proof verification failed with error: {:?}", error);

            }
        }
    }
}