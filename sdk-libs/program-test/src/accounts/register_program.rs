use account_compression::RegisteredProgram;
use light_client::rpc::{errors::RpcError, Rpc};
use light_registry::{
    sdk::{create_deregister_program_instruction, create_register_program_instruction},
    utils::{get_cpi_authority_pda, get_protocol_config_pda_address},
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
};

pub async fn register_program_with_registry_program<R: Rpc>(
    rpc: &mut R,
    governance_authority: &Keypair,
    group_pda: &Pubkey,
    program_id_keypair: &Keypair,
) -> Result<Pubkey, RpcError> {
    let governance_authority_pda = get_protocol_config_pda_address();
    let (instruction, token_program_registered_program_pda) = create_register_program_instruction(
        governance_authority.pubkey(),
        governance_authority_pda,
        *group_pda,
        program_id_keypair.pubkey(),
    );
    let cpi_authority_pda = light_registry::utils::get_cpi_authority_pda();
    let transfer_instruction = system_instruction::transfer(
        &governance_authority.pubkey(),
        &cpi_authority_pda.0,
        rpc.get_minimum_balance_for_rent_exemption(RegisteredProgram::LEN)
            .await
            .unwrap(),
    );

    rpc.create_and_send_transaction(
        &[transfer_instruction, instruction],
        &governance_authority.pubkey(),
        &[governance_authority, program_id_keypair],
    )
    .await?;
    Ok(token_program_registered_program_pda)
}

pub async fn deregister_program_with_registry_program<R: Rpc>(
    rpc: &mut R,
    governance_authority: &Keypair,
    group_pda: &Pubkey,
    program_id_keypair: &Keypair,
) -> Result<Pubkey, RpcError> {
    let governance_authority_pda = get_protocol_config_pda_address();
    let (instruction, token_program_registered_program_pda) = create_deregister_program_instruction(
        governance_authority.pubkey(),
        governance_authority_pda,
        *group_pda,
        program_id_keypair.pubkey(),
    );
    let cpi_authority_pda = get_cpi_authority_pda();
    let transfer_instruction = system_instruction::transfer(
        &governance_authority.pubkey(),
        &cpi_authority_pda.0,
        rpc.get_minimum_balance_for_rent_exemption(RegisteredProgram::LEN)
            .await
            .unwrap(),
    );

    rpc.create_and_send_transaction(
        &[transfer_instruction, instruction],
        &governance_authority.pubkey(),
        &[governance_authority],
    )
    .await?;
    Ok(token_program_registered_program_pda)
}
