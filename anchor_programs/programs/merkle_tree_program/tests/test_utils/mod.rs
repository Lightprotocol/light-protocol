#[cfg(test)]
#[allow(dead_code)]
pub mod tests {
    use ark_bn254;
    use ark_ed_on_bn254;
    use ark_std::vec::Vec;

    use ark_ec;
    use ark_ff::bytes::FromBytes;
    use ark_ff::Fp256;
    use ark_ff::QuadExtField;
    use ark_groth16::{prepare_inputs, prepare_verifying_key};

    use ark_ec::*;
    use light_protocol_program::process_instruction;
    use serde_json::{Result, Value};
    use solana_program::program_option::COption;
    use solana_program::program_pack::Pack;
    use solana_program::pubkey::Pubkey;
    use solana_program::sysvar::rent::Rent;
    use solana_program_test::ProgramTest;
    use solana_program_test::ProgramTestContext;
    use solana_program_test::*;
    use solana_sdk::account::Account;
    use solana_sdk::account::WritableAccount;
    use solana_sdk::stake_history::Epoch;
    use std::fs;
    use std::str::FromStr;

    const ACCOUNT_RENT_EXEMPTION: u64 = 1000000000000u64;

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
            is_native: COption::Some(balance),
            ..spl_token::state::Account::default()
        };
        let mut amount = balance;
        if amount < Rent::minimum_balance(&solana_sdk::sysvar::rent::Rent::default(), 165) {
            amount = Rent::minimum_balance(&solana_sdk::sysvar::rent::Rent::default(), 165)
        }
        Pack::pack(token_account_state, &mut token_account_data).unwrap();
        let token_account = Account::create(
            amount, //ACCOUNT_RENT_EXEMPTION,
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
            let mut account = Account::new(
                Rent::minimum_balance(&solana_sdk::sysvar::rent::Rent::default(), *size),
                *size,
                &program_id,
            );
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
        program_context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

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
        let program_context_new = create_and_start_program_var(
            &accounts_vector,
            token_accounts,
            &program_id,
            &signer_pubkey,
        )
        .await;

        program_context_new
    }
}
