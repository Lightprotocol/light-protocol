use crate::verifying_keypublic_psp_2_in_2_out::VERIFYINGKEY_PUBLIC_PROGRAM_TRANSACTION2_IN2_OUT_MAIN;
use crate::verifying_keypublic_psp_8_in_2_out::VERIFYINGKEY_PUBLIC_TRANSACTION8_IN2_OUT_MAIN;
use anchor_lang::prelude::*;
use light_macros::light_verifier_accounts;
use light_verifier_sdk::{
    light_transaction::ProofCompressed,
    public_transaction::{
        Amounts, PublicTransaction, PublicTransactionInput, PublicTransactionPublicInputs,
        PublicTransactionPublicInputsTransfer,
    },
    utxo::Utxo,
};

#[account]
#[derive(Debug, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub struct u256 {
    pub data: [u8; 32],
}

#[account]
#[derive(Debug, PartialEq, Eq)]
pub struct TransferOutputUtxo {
    pub owner: u256,
    pub amounts: [u64; 2],
    pub spl_asset_mint: Option<Pubkey>,
    pub meta_hash: Option<u256>,
    pub address: Option<u256>,
}

pub fn from_transfer_output_utxo<'a>(utxo: TransferOutputUtxo) -> Utxo {
    // beet big number deserialiazation is little endian
    let mut owner = utxo.owner.data.clone();
    owner.reverse();
    Utxo {
        version: 0,
        pool_type: 0,
        amounts: utxo.amounts,
        spl_asset_mint: Some(utxo.spl_asset_mint.unwrap_or_default()),
        owner,
        blinding: [0u8; 32],
        data_hash: [0u8; 32],
        meta_hash: utxo.meta_hash.unwrap_or(u256 { data: [0u8; 32] }).data,
        address: utxo.address.unwrap_or(u256 { data: [0u8; 32] }).data,
        message: None,
    }
}

pub fn process_2in2out_transfer<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: Vec<u8>,
    test_state_roots: Option<[[u8; 32]; 2]>,
) -> Result<()> {
    let inputs: InstructionDataTransfer2In2Out =
        InstructionDataTransfer2In2Out::try_deserialize_unchecked(
            &mut [vec![0u8; 8], inputs].concat().as_slice(),
        )?;
    msg!("in_utxo_hashes {:?}", inputs.in_utxo_hashes);
    // TODO: refactor into generic function to reuse for input validation in 8in2out
    if inputs.low_element_indexes.len() > 2
        && inputs.low_element_indexes.len() != inputs.in_utxo_hashes.len()
    {
        msg!("number of low element indexes invalid {} > 2 or not equal to number of in utxo hashes {} != {}", inputs.low_element_indexes.len(),inputs.low_element_indexes.len(),  inputs.in_utxo_hashes.len());
        panic!();
    }
    if inputs.out_utxo.len() > 2 {
        msg!("number of out_utxo invalid {} > 2", inputs.out_utxo.len());
        panic!();
    }
    if inputs.in_utxo_hashes.len() > 2 {
        msg!(
            "number of in_utxo_hashes invalid {} > 2",
            inputs.in_utxo_hashes.len()
        );
        panic!();
    }

    let proof = ProofCompressed {
        a: inputs.proof_a,
        b: inputs.proof_b,
        c: inputs.proof_c,
    };

    let mut out_utxos: Vec<Utxo> = Vec::new();
    let mut merkle_root_indexes = [0usize; 2];
    for (i, utxo) in inputs.out_utxo.iter().enumerate() {
        if utxo.is_some() {
            let utxo = utxo.as_ref().unwrap();
            // TODO: optimize vec usage
            let deserialized_utxo: TransferOutputUtxo =
                TransferOutputUtxo::try_deserialize_unchecked(
                    &mut [vec![0u8; 8], utxo.to_vec()].concat().as_slice(),
                )
                .unwrap();
            out_utxos.push(from_transfer_output_utxo(deserialized_utxo));
            merkle_root_indexes[i] = inputs.root_indexes[i].unwrap() as usize;
        }
    }

    let public_amount = Amounts {
        sol: inputs.public_amount_sol,
        spl: inputs.public_amount_spl,
    };
    // let mut low_element_indexes = [0u16; 2];
    // for (i, index) in inputs.low_element_indexes.iter().enumerate() {
    //     low_element_indexes[i] = *index;
    // }
    let input = PublicTransactionInput {
        ctx: &ctx,
        message: None,
        proof: &proof,
        public_amount: Some(&public_amount),
        in_utxo_hashes: &inputs.in_utxo_hashes,
        in_utxo_data_hashes: [None, None],
        out_utxos: out_utxos.clone(),
        merkle_root_indexes,
        rpc_fee: inputs.rpc_fee,
        pool_type: &[0u8; 32],
        verifyingkey: &VERIFYINGKEY_PUBLIC_PROGRAM_TRANSACTION2_IN2_OUT_MAIN,
        program_id: None,
        new_addresses: &[None, None],
        transaction_hash: None,
        low_element_indexes: &inputs.low_element_indexes,
    };
    let mut transaction = PublicTransaction::<
        0,
        2,
        2,
        14,
        TransferInstruction<'info>,
        PublicTransactionPublicInputs<2, 2>,
    >::new(input);

    // this is only for testing
    #[cfg(not(target_os = "solana"))]
    {
        transaction.tx_integrity_hash = [0u8; 32];
        transaction.state_merkle_roots = test_state_roots.unwrap();
        transaction.out_utxo_hashes =
            vec![out_utxos[0].hash().unwrap(), out_utxos[1].hash().unwrap()];
        transaction.mint_pubkey = [
            0, 24, 59, 207, 17, 191, 51, 84, 25, 96, 177, 164, 233, 142, 128, 208, 115, 82, 0, 223,
            237, 121, 0, 231, 241, 213, 140, 224, 58, 185, 152, 253,
        ];
        transaction.verify()?;
    }

    #[cfg(target_os = "solana")]
    transaction.transact()?;
    Ok(())
}

pub fn process_8in2out_transfer<'a, 'b, 'c, 'info: 'b + 'c>(
    ctx: Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    inputs: Vec<u8>,
    test_state_roots: Option<[[u8; 32]; 8]>,
) -> Result<()> {
    let inputs: InstructionDataTransfer8In2Out =
        InstructionDataTransfer8In2Out::try_deserialize_unchecked(
            &mut [vec![0u8; 8], inputs.to_vec()].concat().as_slice(),
        )?;

    let proof = ProofCompressed {
        a: inputs.proof_a,
        b: inputs.proof_b,
        c: inputs.proof_c,
    };

    let out_utxos: Vec<Utxo> = inputs
        .out_utxo
        .map(|x| match x {
            Some(utxo) => {
                let inputs: TransferOutputUtxo = TransferOutputUtxo::try_deserialize_unchecked(
                    &mut [vec![0u8; 8], utxo].concat().as_slice(),
                )
                .unwrap();
                from_transfer_output_utxo(inputs)
            }
            None => Utxo::default(),
        })
        .into_iter()
        .collect::<Vec<_>>();

    let public_amount = Amounts {
        sol: inputs.public_amount_sol,
        spl: inputs.public_amount_spl,
    };

    let input = PublicTransactionInput::<0, 2, 8, TransferInstruction<'info>> {
        ctx: &ctx,
        message: None,
        proof: &proof,
        public_amount: Some(&public_amount),
        in_utxo_hashes: &inputs.in_utxo_hashes,
        in_utxo_data_hashes: [None; 8],
        out_utxos: out_utxos.clone(),
        merkle_root_indexes: [0usize; 8],
        rpc_fee: inputs.rpc_fee,
        pool_type: &[0u8; 32],
        verifyingkey: &VERIFYINGKEY_PUBLIC_TRANSACTION8_IN2_OUT_MAIN,
        program_id: None,
        new_addresses: &[None, None],
        transaction_hash: None,
        low_element_indexes: &inputs.low_element_indexes,
    };
    let mut transaction = PublicTransaction::<
        0,
        2,
        8,
        19,
        TransferInstruction<'info>,
        PublicTransactionPublicInputsTransfer<8, 2>,
    >::new(input);

    // this is only for testing
    #[cfg(not(target_os = "solana"))]
    {
        transaction.tx_integrity_hash = [0u8; 32];
        transaction.state_merkle_roots = test_state_roots.unwrap();
        transaction.out_utxo_hashes = out_utxos
            .iter()
            .map(|utxo| utxo.hash().unwrap())
            .collect::<Vec<[u8; 32]>>()
            .try_into()
            .unwrap();
        transaction.mint_pubkey = [
            0, 24, 59, 207, 17, 191, 51, 84, 25, 96, 177, 164, 233, 142, 128, 208, 115, 82, 0, 223,
            237, 121, 0, 231, 241, 213, 140, 224, 58, 185, 152, 253,
        ];
        transaction.verify()?;
    }
    let x = psp_account_compression::program::PspAccountCompression::id();
    #[cfg(target_os = "solana")]
    transaction.transact()?;
    Ok(())
}

#[light_verifier_accounts(public)]
#[derive(Accounts)]
pub struct TransferInstruction<'info> {}

#[derive(Debug)]
#[account]
pub struct InstructionDataTransfer2In2Out {
    proof_a: [u8; 32],
    proof_b: [u8; 64],
    proof_c: [u8; 32],
    public_amount_spl: Option<[u8; 32]>,
    in_utxo_hashes: Vec<[u8; 32]>,
    low_element_indexes: Vec<u16>, // currently not used just a placeholder value
    public_amount_sol: Option<[u8; 32]>,
    root_indexes: [Option<u64>; 2],
    rpc_fee: Option<u64>,
    out_utxo: [Option<Vec<u8>>; 2],
}

#[derive(Debug)]
#[account]
pub struct InstructionDataTransfer8In2Out {
    proof_a: [u8; 32],
    proof_b: [u8; 64],
    proof_c: [u8; 32],
    public_amount_spl: Option<[u8; 32]>,
    in_utxo_hashes: Vec<[u8; 32]>,
    low_element_indexes: Vec<u16>, // currently not used just a placeholder value
    public_amount_sol: Option<[u8; 32]>,
    root_indexes: [Option<u64>; 8],
    rpc_fee: Option<u64>,
    out_utxo: [Option<Vec<u8>>; 2],
}

/*
#[cfg(test)]
mod test {

    use crate::PROGRAM_ID;

    use super::*;
    use anchor_lang::{prelude::Pubkey, solana_program::system_program};
    use base64::Engine;
    use light_merkle_tree_program::program::LightMerkleTreeProgram;

    use base64::engine::general_purpose;
    // use light_verifier_sdk::utxo::u256;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::{self, Read};
    use std::path::Path;
    use std::str::FromStr;

    fn test_public_2in2out_transfer_p<'a>() -> &'a Pubkey {
        let p = Box::new(Pubkey::from_str("9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE").unwrap());
        let p_ref: &'a Pubkey = Box::leak(p);
        p_ref
    }
    fn get_system_program_id<'a>() -> &'a Pubkey {
        let p = Box::new(system_program::ID);
        let p_ref: &'a Pubkey = Box::leak(p);
        p_ref
    }

    // fn get_json_data(file_path: &str) -> Vec<u8> {
    //     let json_data = fs::read_to_string(file_path).unwrap();
    //     let json_data: JsonAccountData = serde_json::from_str(&json_data).unwrap();
    //     let data = base64::decode(json_data.account.data[0].as_str()).unwrap();
    //     data
    // }

    #[derive(Serialize, Deserialize)]
    struct JsonAccountData {
        pubkey: String,
        account: JsonAccount,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize)]
    struct JsonAccount {
        lamports: u64,
        data: Vec<String>, // Assuming the first element is data in base64, second element is "base64"
        owner: String,
        executable: bool,
        rentEpoch: u64,
        space: u64,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ProofCompressed {
        proofA: [u8; 32],
        proofB: Vec<u8>,
        proofC: [u8; 32],
    }
    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct TestInputs {
        parsedProof: ProofCompressed,
        parsedPublicInputsObject: ParsedPublicInputsObject,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Debug)]
    struct ParsedPublicInputsObject {
        publicStateRoot: Vec<[u8; 32]>,
        publicAmountSpl: Option<[u8; 32]>,
        publicDataHash: [u8; 32],
        publicAmountSol: Option<[u8; 32]>,
        publicMintPublicKey: Option<[u8; 32]>,
        publicInUtxoHash: Vec<[u8; 32]>,
        publicOutUtxoHash: Vec<[u8; 32]>,
        publicNewAddress: Option<Vec<[u8; 32]>>,
        publicInUtxoDataHash: Option<Vec<[u8; 32]>>,
    }

    fn parse_test_data_json_file<P: AsRef<Path>>(path: P) -> TestInputs {
        let file = fs::File::open(path).expect("Unable to open file");
        let mut buf_reader = io::BufReader::new(file);
        let mut contents = String::new();
        buf_reader
            .read_to_string(&mut contents)
            .expect("Unable to read the file");
        serde_json::from_str(&contents).unwrap()
    }
    pub struct AnchorTestEnv<'info> {
        // program_id: Pubkey,
        _light_accounts: Option<TransferInstruction<'info>>,
        // ctx: Option<Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>>,
        system_program_id: Pubkey,
        system_program_id1: Pubkey,
        owner2: Pubkey,
        lamports: u64,
        lamports1: u64,
        lamports2: u64,
        lamports3: u64,
        // lamports4: u64,
        // lamports5: u64,
        merkle_tree_pubkey: Pubkey,
        data: [u8; 8],
        data1: [u8; 8],
        data2: [u8; 8],
        data_registered_verifier: Vec<u8>,
        // state_merkle_tree_data: Vec<u8>,
        // event_merkle_tree_data: Vec<u8>,
    }

    impl<'info> AnchorTestEnv<'info> {
        pub fn new() -> Self {
            // let state_merkle_tree_data = get_json_data(
            //     "../../cli/accounts/transaction-merkle-tree/transaction-merkle-tree.json",
            // );
            // let event_merkle_tree_data =
            //     get_json_data("../../cli/accounts/misc/eventMerkleTreePubkey.json");
            let data_registered_verifier = general_purpose::STANDARD
                .decode(b"V4NKI1y2X63+ZsVtTWVMRkzg9+CJ7wNpuB2guj27kCpL0r0qSL6Cgw==")
                .unwrap();

            Self {
                // program_id: Pubkey::from_str(PROGRAM_ID).unwrap(),
                _light_accounts: None,
                system_program_id: system_program::ID,
                system_program_id1: system_program::ID,
                owner2: LightMerkleTreeProgram::id(),
                // ctx: None,
                lamports: 1_000_000_000u64,
                lamports1: 1_000_000_000u64,
                lamports2: 1_000_000_000u64,
                lamports3: 1_000_000_000u64,
                // lamports4: 1_000_000_000u64,
                // lamports5: 1_000_000_000u64,
                merkle_tree_pubkey: LightMerkleTreeProgram::id(),
                data: [0u8; 8],
                data1: [0u8; 8],
                data2: [0u8; 8],
                data_registered_verifier,
                // state_merkle_tree_data,
                // event_merkle_tree_data,
            }
        }

        pub fn call(&'info mut self) -> TransferInstruction<'info> {
            let signer_pubkey: &'info Pubkey = test_public_2in2out_transfer_p::<'info>();
            let signer_account_info = AccountInfo::new(
                &signer_pubkey,
                true,
                false,
                &mut self.lamports,
                &mut self.data,
                &self.system_program_id,
                false,
                0,
            );
            let signer: Signer = Signer::try_from(&signer_account_info).unwrap();

            let program_id: &'info Pubkey = get_system_program_id::<'info>();
            // let system_program_id : &'info Pubkey= &self.system_program_id;
            let system_program_account_info = AccountInfo::new(
                &program_id,
                false,
                false,
                &mut self.lamports1,
                &mut self.data1,
                &self.system_program_id1,
                true,
                0,
            );

            let system_program = Program::<System>::try_from(&system_program_account_info).unwrap();
            let program_merkle_tree_account_info = AccountInfo::new(
                &self.merkle_tree_pubkey,
                false,
                false,
                &mut self.lamports2,
                &mut self.data2,
                &self.owner2,
                true,
                0,
            );

            let program_merkle_tree =
                Program::<LightMerkleTreeProgram>::try_from(&program_merkle_tree_account_info)
                    .unwrap();

            let register_verifier_account_info = AccountInfo::new(
                &signer_pubkey,
                false,
                false,
                &mut self.lamports3,
                &mut self.data_registered_verifier,
                &self.owner2,
                false,
                0,
            );
            // let state_merkle_tree_account_info = AccountInfo::new(
            //     &self.merkle_tree_pubkey,
            //     false,
            //     true,
            //     &mut self.lamports4,
            //     &mut self.state_merkle_tree_data,
            //     &self.owner2,
            //     false,
            //     0,
            // );
            // let event_merkle_tree_account_info = AccountInfo::new(
            //     &self.merkle_tree_pubkey,
            //     false,
            //     true,
            //     &mut self.lamports5,
            //     &mut self.event_merkle_tree_data,
            //     &self.owner2,
            //     false,
            //     0,
            // );

            let unchecked_dummy_account =
                UncheckedAccount::try_from(register_verifier_account_info.clone());
            let light_accounts = TransferInstruction {
                signing_address: signer,
                system_program,
                program_merkle_tree: program_merkle_tree,
                registered_verifier_pda: Account::try_from(&register_verifier_account_info.clone())
                    .unwrap(),
                rpc_recipient_sol: unchecked_dummy_account.clone(),
                log_wrapper: unchecked_dummy_account.clone(),
                authority: unchecked_dummy_account.clone(),
            };
            light_accounts
        }
    }
    fn transact2in2out<'a, 'b, 'c, 'info: 'b + 'c>(
        instruction_data: Vec<u8>,
        program_id: &'a Pubkey,
        light_accounts: &'info mut TransferInstruction<'info>,
        test_inputs: &TestInputs,
    ) {
        let ctx = Context::<'a, 'b, 'c, 'info, TransferInstruction<'info>>::new(
            program_id,
            light_accounts,
            &[],
            BTreeMap::new(),
        );

        process_2in2out_transfer(
            ctx,
            instruction_data,
            Some(
                test_inputs
                    .parsedPublicInputsObject
                    .publicStateRoot
                    .clone()
                    .try_into()
                    .unwrap(),
            ),
        )
        .unwrap();
    }

    fn transact_8in2out<'a, 'b, 'c, 'info: 'b + 'c>(
        instruction_data: Vec<u8>,
        program_id: &'a Pubkey,
        light_accounts: &'info mut TransferInstruction<'info>,
        test_inputs: TestInputs,
    ) {
        let ctx = Context::<'a, 'b, 'c, 'info, TransferInstruction<'info>>::new(
            program_id,
            light_accounts,
            &[],
            BTreeMap::new(),
        );

        process_8in2out_transfer(
            ctx,
            instruction_data,
            Some(
                test_inputs
                    .parsedPublicInputsObject
                    .publicStateRoot
                    .clone()
                    .try_into()
                    .unwrap(),
            ),
        )
        .unwrap();
    }
    #[test]
    pub fn test_public_8in2out_transfer() {
        let test_inputs = parse_test_data_json_file("./test-data/public8in2out.json");
        let rpc_fee = 0u64;

        let owner: u256 = u256 {
            data: [
                9, 203, 246, 71, 191, 105, 220, 115, 150, 213, 89, 159, 251, 168, 170, 12, 109,
                193, 195, 87, 133, 235, 169, 26, 45, 163, 109, 188, 150, 25, 27, 4,
            ],
        };
        let spl_asset_mint =
            Some(Pubkey::from_str("ycrF6Bw3doNPMSDmZM1rxNHimD2bwq1UFmifMCzbjAe").unwrap());
        let mut utxo_0_bytes = Vec::new();
        TransferOutputUtxo {
            amounts: [5000, 0],
            spl_asset_mint,
            meta_hash: Some(u256::default()),
            address: Some(u256::default()),
            owner: owner.clone(),
        }
        .serialize(&mut utxo_0_bytes)
        .unwrap();
        let mut utxo_1_bytes = Vec::new();
        TransferOutputUtxo {
            amounts: [5000, 0],
            spl_asset_mint,
            meta_hash: Some(u256::default()),
            address: Some(u256::default()),
            owner,
        }
        .serialize(&mut utxo_1_bytes)
        .unwrap();
        let instruction_data = InstructionDataTransfer8In2Out {
            proof_a: test_inputs.parsedProof.proofA,
            proof_b: test_inputs.parsedProof.proofB.clone().try_into().unwrap(),
            proof_c: test_inputs.parsedProof.proofC,
            public_amount_spl: test_inputs.parsedPublicInputsObject.publicAmountSpl,
            in_utxo_hashes: [
                Some(test_inputs.parsedPublicInputsObject.publicInUtxoHash[0]),
                Some(test_inputs.parsedPublicInputsObject.publicInUtxoHash[1]),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            public_amount_sol: test_inputs.parsedPublicInputsObject.publicAmountSol,
            root_indexes: [Some(0u64), Some(0u64), None, None, None, None, None, None],
            rpc_fee: Some(rpc_fee),
            out_utxo: [Some(utxo_0_bytes), Some(utxo_1_bytes)],
        };
        let instruction_data = instruction_data.try_to_vec().unwrap();
        let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
        let mut test_helper = AnchorTestEnv::new();

        let mut light_accounts = test_helper.call();
        transact_8in2out(
            instruction_data,
            &program_id,
            &mut light_accounts,
            test_inputs,
        )
    }
    #[test]
    pub fn test_public_2in2out_transfer() {
        let test_inputs = parse_test_data_json_file("./test-data/public2in2out.json");
        let rpc_fee = 0u64;

        let owner: u256 = u256 {
            data: [
                9, 203, 246, 71, 191, 105, 220, 115, 150, 213, 89, 159, 251, 168, 170, 12, 109,
                193, 195, 87, 133, 235, 169, 26, 45, 163, 109, 188, 150, 25, 27, 4,
            ],
        };
        let spl_asset_mint =
            Some(Pubkey::from_str("ycrF6Bw3doNPMSDmZM1rxNHimD2bwq1UFmifMCzbjAe").unwrap());
        let mut utxo_0_bytes = Vec::new();
        TransferOutputUtxo {
            amounts: [5000, 10_000],
            spl_asset_mint,
            meta_hash: Some(u256::default()),
            address: Some(u256::default()),
            owner: owner.clone(),
        }
        .serialize(&mut utxo_0_bytes)
        .unwrap();
        let mut utxo_1_bytes = Vec::new();
        TransferOutputUtxo {
            amounts: [5000, 10_000],
            spl_asset_mint,
            meta_hash: Some(u256::default()),
            address: Some(u256::default()),
            owner,
        }
        .serialize(&mut utxo_1_bytes)
        .unwrap();
        let instruction_data = InstructionDataTransfer2In2Out {
            proof_a: test_inputs.parsedProof.proofA,
            proof_b: test_inputs.parsedProof.proofB.clone().try_into().unwrap(),
            proof_c: test_inputs.parsedProof.proofC,
            public_amount_spl: test_inputs.parsedPublicInputsObject.publicAmountSpl,
            in_utxo_hashes: [
                Some(test_inputs.parsedPublicInputsObject.publicInUtxoHash[0]),
                Some(test_inputs.parsedPublicInputsObject.publicInUtxoHash[1]),
            ],
            public_amount_sol: test_inputs.parsedPublicInputsObject.publicAmountSol,
            root_indexes: [Some(0u64), Some(0u64)],
            rpc_fee: Some(rpc_fee),
            out_utxo: [Some(utxo_0_bytes), Some(utxo_1_bytes)],
        };
        let instruction_data = instruction_data.try_to_vec().unwrap();
        let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();
        let mut test_helper = AnchorTestEnv::new();

        let mut light_accounts = test_helper.call();
        transact2in2out(
            instruction_data,
            &program_id,
            &mut light_accounts,
            &test_inputs,
        )
    }
}
*/
