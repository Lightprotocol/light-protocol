use litesvm::LiteSVM;
use solana_account::ReadableAccount;
use solana_sdk::{account::Account, pubkey::Pubkey};

/// Extension trait for LiteSVM to add utility methods.
///
/// This trait provides additional functionality on top of the base LiteSVM,
/// such as querying all accounts owned by a specific program.
pub trait LiteSvmExtensions {
    /// Returns all accounts owned by the provided program id.
    ///
    /// This method iterates through the internal accounts database and filters
    /// accounts by their owner field.
    ///
    /// # Arguments
    ///
    /// * `program_id` - The program ID to filter accounts by
    ///
    /// # Returns
    ///
    /// A vector of tuples containing (Pubkey, Account) for all accounts owned by the program
    fn get_program_accounts(&self, program_id: &Pubkey) -> Vec<(Pubkey, Account)>;
}

impl LiteSvmExtensions for LiteSVM {
    fn get_program_accounts(&self, program_id: &Pubkey) -> Vec<(Pubkey, Account)> {
        self.accounts_db()
            .inner
            .iter()
            .filter(|(_, account)| account.owner() == program_id)
            .map(|(pubkey, account)| (*pubkey, Account::from(account.clone())))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::{
        native_token::LAMPORTS_PER_SOL,
        signature::{Keypair, Signer},
        system_instruction::{create_account, transfer},
        system_program,
        transaction::{Transaction, VersionedTransaction},
    };

    use super::*;

    #[test]
    fn test_get_program_accounts() {
        let mut svm = LiteSVM::new();
        let payer_keypair = Keypair::new();
        let payer = payer_keypair.pubkey();

        // Fund the payer
        svm.airdrop(&payer, 10 * LAMPORTS_PER_SOL).unwrap();

        // Establish baseline of system accounts
        let baseline_system_accounts = svm.get_program_accounts(&system_program::id());
        let baseline_count = baseline_system_accounts.len();

        // Create multiple accounts owned by system program
        let num_system_accounts = 5;
        let mut system_owned_accounts = vec![];

        for i in 0..num_system_accounts {
            let new_account_keypair = Keypair::new();
            let new_account = new_account_keypair.pubkey();
            let space = 10 + i;
            let rent_amount = svm.minimum_balance_for_rent_exemption(space);

            let instruction = create_account(
                &payer,
                &new_account,
                rent_amount,
                space as u64,
                &system_program::id(),
            );

            let tx = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&payer),
                &[&payer_keypair, &new_account_keypair],
                svm.latest_blockhash(),
            );

            svm.send_transaction(VersionedTransaction::from(tx))
                .unwrap();
            system_owned_accounts.push((new_account, rent_amount, space));
        }

        // Create a custom program and some accounts owned by it
        let custom_program_id = Pubkey::new_unique();
        let num_custom_accounts = 3;
        let mut custom_owned_accounts = vec![];

        for i in 0..num_custom_accounts {
            let new_account_keypair = Keypair::new();
            let new_account = new_account_keypair.pubkey();
            let space = 20 + i * 2;
            let rent_amount = svm.minimum_balance_for_rent_exemption(space);

            let instruction = create_account(
                &payer,
                &new_account,
                rent_amount,
                space as u64,
                &custom_program_id,
            );

            let tx = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&payer),
                &[&payer_keypair, &new_account_keypair],
                svm.latest_blockhash(),
            );

            svm.send_transaction(VersionedTransaction::from(tx))
                .unwrap();
            custom_owned_accounts.push((new_account, rent_amount, space));
        }

        // Do some transfers to create new accounts
        for i in 0..3 {
            let to = Pubkey::new_unique();
            let amount = (i + 1) * 1000;
            let instruction = transfer(&payer, &to, amount);
            let tx = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&payer),
                &[&payer_keypair],
                svm.latest_blockhash(),
            );
            svm.send_transaction(VersionedTransaction::from(tx))
                .unwrap();
        }

        // Test get_program_accounts for system program
        let system_accounts = svm.get_program_accounts(&system_program::id());

        // Should contain baseline + 5 created accounts + 3 transfer recipients
        let expected_count = baseline_count + num_system_accounts + 3;
        assert_eq!(
            system_accounts.len(),
            expected_count,
            "Expected {} system accounts (baseline {} + {} created + 3 transfers), got {}",
            expected_count,
            baseline_count,
            num_system_accounts,
            system_accounts.len()
        );

        // Verify all our created system accounts are present with correct data
        for (pubkey, expected_lamports, expected_space) in &system_owned_accounts {
            let found = system_accounts
                .iter()
                .find(|(pk, _)| pk == pubkey)
                .expect("System account should be found");

            assert_eq!(found.1.lamports, *expected_lamports);
            assert_eq!(found.1.data.len(), *expected_space);
            assert_eq!(found.1.owner, system_program::id());
        }

        // Verify individual get_account returns the same data
        for (pubkey, _, _) in &system_owned_accounts {
            let individual_account = svm.get_account(pubkey).unwrap();
            let from_program_accounts =
                system_accounts.iter().find(|(pk, _)| pk == pubkey).unwrap();

            assert_eq!(
                individual_account.lamports,
                from_program_accounts.1.lamports
            );
            assert_eq!(individual_account.data, from_program_accounts.1.data);
            assert_eq!(individual_account.owner, from_program_accounts.1.owner);
        }

        // Test get_program_accounts for custom program
        let custom_accounts = svm.get_program_accounts(&custom_program_id);
        assert_eq!(custom_accounts.len(), num_custom_accounts);

        // Verify all custom accounts are present with correct data
        for (pubkey, expected_lamports, expected_space) in &custom_owned_accounts {
            let found = custom_accounts
                .iter()
                .find(|(pk, _)| pk == pubkey)
                .expect("Custom account should be found");

            assert_eq!(found.1.lamports, *expected_lamports);
            assert_eq!(found.1.data.len(), *expected_space);
            assert_eq!(found.1.owner, custom_program_id);
        }

        // Test get_program_accounts for non-existent program
        let nonexistent_program = Pubkey::new_unique();
        let no_accounts = svm.get_program_accounts(&nonexistent_program);
        assert_eq!(no_accounts.len(), 0);
    }
}
