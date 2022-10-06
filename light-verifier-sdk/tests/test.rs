mod verification_key;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::groth16::{Groth16Verifyingkey, Groth16Verifier};
    use ark_ff::bytes::{ToBytes, FromBytes};
    use ark_ec;
    use ark_bn254;
    use std::ops::Neg;
    use verification_key::VERIFYINGKEY;

    type G1 = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>;

    fn to_be_64(bytes: &[u8]) -> Vec<u8> {
        let mut vec = Vec::new();
        for b in bytes.chunks(32) {
            for byte in b.iter().rev() {
                vec.push(*byte);
            }
        }
        vec
    }

    pub const PUBLIC_INPUTS: [u8; 9 * 32] = [34,238,251,182,234,248,214,189,46,67,42,25,71,58,145,58,61,28,116,110,60,17,82,149,178,187,160,211,37,226,174,231,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,51,152,17,147,4,247,199,87,230,85,103,90,28,183,95,100,200,46,3,158,247,196,173,146,207,167,108,33,199,18,13,204,198,101,223,186,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,7,49,65,41,7,130,55,65,197,232,175,217,44,151,149,225,75,86,158,105,43,229,65,87,51,150,168,243,176,175,11,203,180,149,72,103,46,93,177,62,42,66,223,153,51,193,146,49,154,41,69,198,224,13,87,80,222,171,37,141,0,1,50,172,18,28,213,213,40,141,45,3,180,200,250,112,108,94,35,143,82,63,125,9,147,37,191,75,62,221,138,20,166,151,219,237,254,58,230,189,33,100,143,241,11,251,73,141,229,57,129,168,83,23,235,147,138,225,177,250,13,97,226,162,6,232,52,95,128,84,90,202,25,178,1,208,219,169,222,123,113,202,165,77,183,98,103,237,187,93,178,95,169,156,38,100,125,218,104,94,104,119,13,21];

    pub const PROOF: [u8; 256] = [45,206,255,166,152,55,128,138,79,217,145,164,25,74,120,234,234,217,68,149,162,44,133,120,184,205,12,44,175,98,168,172,20,24,216,15,209,175,106,75,147,236,90,101,123,219,245,151,209,202,218,104,148,8,32,254,243,191,218,122,42,81,193,84,40,57,233,205,180,46,35,111,215,5,23,93,12,71,118,225,7,46,247,147,47,130,106,189,184,80,146,103,141,52,242,25,0,203,124,176,110,34,151,212,66,180,238,151,236,189,133,209,17,137,205,183,168,196,92,159,75,174,81,168,18,86,176,56,16,26,210,20,18,81,122,142,104,62,251,169,98,141,21,253,50,130,182,15,33,109,228,31,79,183,88,147,174,108,4,22,14,129,168,6,80,246,254,100,218,131,94,49,247,211,3,245,22,200,177,91,60,144,147,174,90,17,19,189,62,147,152,18,41,139,183,208,246,198,118,127,89,160,9,27,61,26,123,180,221,108,17,166,47,115,82,48,132,139,253,65,152,92,209,53,37,25,83,61,252,42,181,243,16,21,2,199,123,96,218,151,253,86,69,181,202,109,64,129,124,254,192,25,177,199,26,50];


    #[test]
    fn proof_verification_should_succeed() {
        let mut public_inputs_vec = Vec::new();
        for input in PUBLIC_INPUTS.chunks(32) {
            public_inputs_vec.push(input.to_vec());
        }

        let proof_a: G1 =  <G1 as FromBytes>::read(&*[&to_be_64(&PROOF[0..64])[..], &[0u8][..]].concat()).unwrap();
        let mut proof_a_neg = [0u8;65];
        <G1 as ToBytes>::write(&proof_a.neg(), &mut proof_a_neg[..]).unwrap();

        let mut verifier = Groth16Verifier::new(
            to_be_64(&proof_a_neg[..64]).to_vec(),
            PROOF[64..192].to_vec(),
            PROOF[192..256].to_vec(),
            public_inputs_vec,
            &VERIFYING_KEY
        ).unwrap();
        verifier.verify().unwrap();
    }

    #[test]
    fn wrong_proof_verification_should_not_succeed() {

        let mut public_inputs_vec = Vec::new();
        for input in PUBLIC_INPUTS.chunks(32) {
            public_inputs_vec.push(input.to_vec());
        }

        let mut verifier = Groth16Verifier::new(
            PROOF[0..64].to_vec(), // using non negated proof a as test for wrong proof
            PROOF[64..192].to_vec(),
            PROOF[192..256].to_vec(),
            public_inputs_vec,
            &VERIFYING_KEY
        ).unwrap();

        assert_eq!(verifier.verify(), Err(Groth16Error::ProofVerificationFailed));
    }

    #[test]
    fn invalid_nr_public_inputs_should_not_succeed() {
        let mut public_inputs_vec = Vec::new();
        for input in PUBLIC_INPUTS.chunks(32) {
            public_inputs_vec.push(input.to_vec());
        }
        public_inputs_vec.push(vec![1u8;32]);

        let verifier = Groth16Verifier::new(
            PROOF[0..64].to_vec(), // using non negated proof a as test for wrong proof
            PROOF[64..192].to_vec(),
            PROOF[192..256].to_vec(),
            public_inputs_vec,
            &VERIFYING_KEY
        );

        assert_eq!(verifier, Err(Groth16Error::InvalidPublicInputsLength));
    }
}
