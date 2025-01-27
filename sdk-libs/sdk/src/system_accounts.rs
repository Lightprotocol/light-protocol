use solana_program::{account_info::AccountInfo, instruction::AccountMeta};

#[repr(usize)]
pub enum LightSystemAccountIndex {
    FeePayer = 0,
    Authority = 1,
    RegisteredProgramPda = 2,
    NoopProgram = 3,
    AccountCompressionAuthority = 4,
    AccountCompressionProgram = 5,
    InvokingProgram = 6,
    SolPoolPda = 7,
    DecompressionRecipient,
    SystemProgram,
    LightSystemProgram,
}

pub const SYSTEM_ACCOUNTS_LEN: usize = 11;

pub struct LightCpiAccounts<'c, 'info> {
    accounts: Vec<&'c AccountInfo<'info>>,
}

impl<'c, 'info> LightCpiAccounts<'c, 'info> {
    pub fn new<I>(
        fee_payer: &'c AccountInfo<'info>,
        authority: &'c AccountInfo<'info>,
        accounts: &'c [I],
    ) -> Self
    where
        I: AsRef<AccountInfo<'info>>,
    {
        let mut cpi_accounts = Vec::with_capacity(accounts.len() + 2);
        cpi_accounts.push(fee_payer);
        cpi_accounts.push(authority);
        cpi_accounts.extend(accounts.iter().map(AsRef::as_ref));
        Self {
            accounts: cpi_accounts,
        }
    }

    pub fn new_with_start_index<I>(
        fee_payer: &'c AccountInfo<'info>,
        authority: &'c AccountInfo<'info>,
        accounts: &'c [I],
        start_index: usize,
    ) -> Self
    where
        I: AsRef<AccountInfo<'info>>,
    {
        let mut cpi_accounts = Vec::with_capacity(SYSTEM_ACCOUNTS_LEN);
        cpi_accounts.push(fee_payer);
        cpi_accounts.push(authority);

        let accounts = &accounts[start_index..];
        let accounts = &accounts[..(SYSTEM_ACCOUNTS_LEN - 2).min(accounts.len())];
        cpi_accounts.extend(accounts.iter().map(AsRef::as_ref));

        Self {
            accounts: cpi_accounts,
        }
    }

    pub fn fee_payer(&self) -> &'c AccountInfo<'info> {
        self.accounts[LightSystemAccountIndex::FeePayer as usize]
    }

    pub fn authority(&self) -> &'c AccountInfo<'info> {
        self.accounts[LightSystemAccountIndex::Authority as usize]
    }

    pub fn invoking_program(&self) -> &'c AccountInfo<'info> {
        self.accounts[LightSystemAccountIndex::InvokingProgram as usize]
    }

    pub fn light_system_program(&self) -> &'c AccountInfo<'info> {
        self.accounts[LightSystemAccountIndex::LightSystemProgram as usize]
    }

    #[inline(always)]
    pub fn setup_cpi_accounts(&self) -> (Vec<AccountInfo<'info>>, Vec<AccountMeta>) {
        let account_infos = self
            .accounts
            .iter()
            .map(|acc| (*acc).clone())
            .collect::<Vec<_>>();
        let account_metas = account_infos
            .iter()
            .enumerate()
            .map(|(i, acc)| {
                let is_signer = i == LightSystemAccountIndex::FeePayer as usize
                    || i == LightSystemAccountIndex::Authority as usize;
                let is_writable = i == LightSystemAccountIndex::FeePayer as usize;
                AccountMeta {
                    pubkey: acc.key.clone(),
                    is_signer,
                    is_writable,
                }
            })
            .collect::<Vec<_>>();

        (account_infos, account_metas)
    }
}
