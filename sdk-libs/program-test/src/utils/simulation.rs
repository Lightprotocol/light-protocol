use crate::program_test::LightProgramTest;
use crate::Rpc;
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
    transaction::{Transaction, VersionedTransaction},
};

/// Simulate a transaction and return the compute units consumed.
///
/// This is a test utility function for measuring transaction costs.
pub async fn simulate_cu(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    instruction: &Instruction,
) -> u64 {
    let blockhash = rpc.get_latest_blockhash().await.unwrap().0;
    let tx = Transaction::new_signed_with_payer(
        &[instruction.clone()],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );
    let simulate_tx = VersionedTransaction::from(tx);

    let simulate_result = rpc.context.simulate_transaction(simulate_tx);
    simulate_result.unwrap().meta.compute_units_consumed
}
