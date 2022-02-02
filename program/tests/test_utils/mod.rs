#[cfg(test)]
pub mod tests {
    use ark_bn254;
    use ark_ed_on_bn254;
    use ark_std::vec::Vec;

    use ark_ec;
    use ark_ff::bytes::FromBytes;
    use ark_ff::Fp256;
    use ark_ff::QuadExtField;
    use ark_groth16::{prepare_inputs, prepare_verifying_key, verify_proof};
    use light_protocol_program::utils::init_bytes18::MERKLE_TREE_ACC_BYTES_ARRAY;

    use ark_ec::*;
    use light_protocol_program::groth16_verifier::final_exponentiation::state::FinalExponentiationState;
    use light_protocol_program::groth16_verifier::parsers::parse_f_to_bytes;
    use light_protocol_program::groth16_verifier::parsers::parse_x_group_affine_from_bytes;
    use light_protocol_program::groth16_verifier::parsers::*;
    use light_protocol_program::process_instruction;
    use serde_json::{Result, Value};
    use solana_program::program_pack::Pack;
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };
    use solana_program_test::ProgramTest;
    use solana_program_test::ProgramTestContext;
    use solana_program_test::*;
    use solana_sdk::account::WritableAccount;
    use solana_sdk::stake_history::Epoch;
    use solana_sdk::{
        account::Account, msg, signature::Signer, transaction::Transaction,
        transport::TransportError,
    };
    use std::fs;
    use std::str::FromStr;

    const ACCOUNT_RENT_EXEMPTION: u64 = 1000000000000u64;
    use solana_sdk::signer::keypair::Keypair;
    pub fn get_ref_value(mode: &str) -> Vec<u8> {
        let bytes;
        let ix_data = read_test_data(String::from("deposit_0_1_sol.txt"));
        let public_inputs_bytes = ix_data[9..233].to_vec(); // 224 length
        let pvk_unprepped = get_vk_from_file().unwrap();
        let pvk = prepare_verifying_key(&pvk_unprepped);
        let public_inputs = get_public_inputs_from_bytes(&public_inputs_bytes).unwrap();
        let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();
        if mode == "prepared_inputs" {
            // We must convert to affine here since the program converts projective into affine already as the last step of prepare_inputs.
            // While the native library implementation does the conversion only when the millerloop is called.
            // The reason we're doing it as part of prepare_inputs is that it takes >1 ix to compute the conversion.
            let as_affine = (prepared_inputs).into_affine();
            let mut affine_bytes = vec![0; 64];
            parse_x_group_affine_to_bytes(as_affine, &mut affine_bytes);
            bytes = affine_bytes;
        } else {
            let proof_bytes = ix_data[233..489].to_vec(); // 256 length
            let proof = get_proof_from_bytes(&proof_bytes);
            let miller_output =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::miller_loop(
                    [
                        (proof.a.into(), proof.b.into()),
                        (
                            (prepared_inputs).into_affine().into(),
                            pvk.gamma_g2_neg_pc.clone(),
                        ),
                        (proof.c.into(), pvk.delta_g2_neg_pc.clone()),
                    ]
                    .iter(),
                );
            if mode == "miller_output" {
                let mut miller_output_bytes = vec![0; 384];
                parse_f_to_bytes(miller_output, &mut miller_output_bytes);
                bytes = miller_output_bytes;
            } else if mode == "final_exponentiation" {
                let res = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::final_exponentiation(&miller_output).unwrap();
                let mut res_bytes = vec![0; 384];
                parse_f_to_bytes(res, &mut res_bytes);
                bytes = res_bytes;
            } else {
                bytes = vec![];
            }
        }
        bytes
    }

    pub fn get_proof_from_bytes(
        proof_bytes: &Vec<u8>,
    ) -> ark_groth16::data_structures::Proof<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
        let proof_a = parse_x_group_affine_from_bytes(&proof_bytes[0..64].to_vec());
        let proof_b = parse_proof_b_from_bytes(&proof_bytes[64..192].to_vec());
        let proof_c = parse_x_group_affine_from_bytes(&proof_bytes[192..256].to_vec());
        let proof =
            ark_groth16::data_structures::Proof::<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
                a: proof_a,
                b: proof_b,
                c: proof_c,
            };
        proof
    }

    pub fn get_public_inputs_from_bytes(
        public_inputs_bytes: &Vec<u8>,
    ) -> Result<Vec<Fp256<ark_ed_on_bn254::FqParameters>>> {
        let mut res = Vec::new();
        for i in public_inputs_bytes.chunks(32) {
            res.push(<Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&i[..]).unwrap())
        }
        Ok(res)
    }

    pub fn get_vk_from_file() -> Result<
        ark_groth16::data_structures::VerifyingKey<ark_ec::models::bn::Bn<ark_bn254::Parameters>>,
    > {
        let contents = fs::read_to_string("./tests/test_data/verification_key_bytes_254.txt")
            .expect("Something went wrong reading the file");
        let v: Value = serde_json::from_str(&contents)?;
        let mut a_g1_bigints = Vec::new();
        for i in 0..2 {
            let mut bytes: Vec<u8> = Vec::new();
            for i in v["vk_alpha_1"][i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
            a_g1_bigints
                .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
        let alpha_g1_bigints = ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
            a_g1_bigints[0],
            a_g1_bigints[1],
            false,
        );

        let mut b_g2_bigints = Vec::new();
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in v["vk_beta_2"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                b_g2_bigints
                    .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }

        let beta_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                b_g2_bigints[0],
                b_g2_bigints[1],
            ),
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                b_g2_bigints[2],
                b_g2_bigints[3],
            ),
            false,
        );

        let mut delta_g2_bytes = Vec::new();
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in v["vk_delta_2"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                delta_g2_bytes
                    .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }

        let delta_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                delta_g2_bytes[0],
                delta_g2_bytes[1],
            ),
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                delta_g2_bytes[2],
                delta_g2_bytes[3],
            ),
            false,
        );

        let mut gamma_g2_bytes = Vec::new();
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in v["vk_gamma_2"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                gamma_g2_bytes
                    .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }
        let gamma_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                gamma_g2_bytes[0],
                gamma_g2_bytes[1],
            ),
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                gamma_g2_bytes[2],
                gamma_g2_bytes[3],
            ),
            false,
        );

        let mut gamma_abc_g1_bigints_bytes = Vec::new();

        for i in 0..8 {
            let mut g1_bytes = Vec::new();
            for u in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in v["IC"][i][u].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                g1_bytes
                    .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
            gamma_abc_g1_bigints_bytes.push(ark_ec::models::bn::g1::G1Affine::<
                ark_bn254::Parameters,
            >::new(g1_bytes[0], g1_bytes[1], false));
        }

        Ok(ark_groth16::data_structures::VerifyingKey::<
            ark_ec::models::bn::Bn<ark_bn254::Parameters>,
        > {
            alpha_g1: alpha_g1_bigints,
            beta_g2: beta_g2,
            gamma_g2: gamma_g2,
            delta_g2: delta_g2,
            gamma_abc_g1: gamma_abc_g1_bigints_bytes,
        })
    }

    pub fn read_test_data(file_name: std::string::String) -> Vec<u8> {
        let mut path = std::string::String::from("./tests/test_data/");
        path.push_str(&file_name);
        println!("reading file: {:?}", path);
        let ix_data_file = fs::read_to_string(path).expect("Something went wrong reading the file");
        let ix_data_json: Value = serde_json::from_str(&ix_data_file).unwrap();
        let mut ix_data = Vec::new();
        for i in ix_data_json["bytes"][0].as_str().unwrap().split(',') {
            let j = (*i).parse::<u8>();
            match j {
                Ok(x) => (ix_data.push(x)),
                Err(_e) => (),
            }
        }
        println!("Appending merkle tree bytes and merkle tree index");
        // for i in 0..32 {
        //     ix_data.push(MERKLE_TREE_ACC_BYTES_ARRAY[0].0[i]);
        // }
        // //pushing merkle tree index
        // ix_data.push(0);

        println!("{:?}", ix_data);
        ix_data
    }

    pub fn add_token_account(
        program_test: &mut ProgramTest,
        mint: Pubkey,
        owner: Pubkey,
        balance: u64,
    ) -> Pubkey {
        let token_account_keypair = Keypair::new();
        add_token_account_with_address(
            program_test,
            token_account_keypair.pubkey(),
            mint,
            owner,
            balance,
        );
        token_account_keypair.pubkey()
    }

    fn add_token_account_with_address(
        program_test: &mut ProgramTest,
        token_address: Pubkey,
        mint: Pubkey,
        owner: Pubkey,
        balance: u64,
    ) {
        let mut token_account_data = vec![0u8; spl_token::state::Account::LEN];
        let token_account_state = spl_token::state::Account {
            mint,
            owner,
            amount: balance,
            state: spl_token::state::AccountState::Initialized,
            ..spl_token::state::Account::default()
        };
        Pack::pack(token_account_state, &mut token_account_data).unwrap();
        let token_account = Account::create(
            ACCOUNT_RENT_EXEMPTION,
            token_account_data,
            spl_token::id(),
            false,
            Epoch::default(),
        );
        program_test.add_account(token_address, token_account);
    }

    pub async fn create_and_start_program_var(
        accounts: &Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
        token_accounts: Option<&mut Vec<(&Pubkey, &Pubkey, u64)>>,
        program_id: &Pubkey,
        signer_pubkey: &Pubkey,
    ) -> ProgramTestContext {
        let mut program_test = ProgramTest::new(
            "light_protocol_program",
            *program_id,
            processor!(process_instruction),
        );
        for (pubkey, size, data) in accounts.iter() {
            let mut account = Account::new(10000000000, *size, &program_id);
            match data {
                Some(d) => (account.data = d.clone()),
                None => (),
            }
            program_test.add_account(**pubkey, account);
            println!("added account {:?}", **pubkey);
        }

        if token_accounts.is_some() {
            let mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

            for (pubkey, authority, amount) in token_accounts.unwrap() {
                add_token_account_with_address(
                    &mut program_test,
                    **pubkey,
                    mint,
                    **authority,
                    *amount,
                );
            }
        }

        let mut program_context = program_test.start_with_context().await;
        //transfer an arbitrary high amount to signer keypair to have a consistent payer
        let mut transaction = solana_sdk::system_transaction::transfer(
            &program_context.payer,
            &signer_pubkey,
            10000000000000,
            program_context.last_blockhash,
        );
        transaction.sign(&[&program_context.payer], program_context.last_blockhash);
        let res_request = program_context
            .banks_client
            .process_transaction(transaction)
            .await;

        program_context
    }

    // We need program restart logic since we're firing 300+ ix and
    // the program_context seems to melt down every couple of hundred ix.
    // It basically just picks up the account state where it left off and restarts the client
    pub async fn restart_program(
        accounts_vector: &mut Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
        token_accounts: Option<&mut Vec<(&Pubkey, &Pubkey, u64)>>,
        program_id: &Pubkey,
        signer_pubkey: &Pubkey,
        program_context: &mut ProgramTestContext,
    ) -> ProgramTestContext {
        for (pubkey, _, current_data) in accounts_vector.iter_mut() {
            let account = program_context
                .banks_client
                .get_account(**pubkey)
                .await
                .expect("get_account")
                .unwrap();
            *current_data = Some(account.data.to_vec());
        }
        let mut program_context_new = create_and_start_program_var(
            &accounts_vector,
            token_accounts,
            &program_id,
            &signer_pubkey,
        )
        .await;

        program_context_new
    }
}
