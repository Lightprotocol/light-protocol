use light_sdk_types::cpi_context_write::CpiContextWriteAccounts;
use solana_account_info::AccountInfo;
use solana_instruction::AccountMeta;

#[cfg(all(feature = "cpi-context", not(feature = "v2")))]
compile_error!(
    "The `cpi-context` feature requires the `v2` feature when using `CpiContextWriteAccounts`. \
     Please enable both features: features = [\"v2\", \"cpi-context\"]"
);

pub fn get_account_metas_from_config_cpi_context(
    config: CpiContextWriteAccounts<AccountInfo>,
) -> [AccountMeta; 3] {
    [
        AccountMeta::new(*config.fee_payer.key, true),
        AccountMeta::new_readonly(config.cpi_signer.cpi_signer.into(), true),
        AccountMeta::new(*config.cpi_context.key, false),
    ]
}
