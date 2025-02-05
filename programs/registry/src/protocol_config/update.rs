use anchor_lang::prelude::*;

use super::state::{ProtocolConfig, ProtocolConfigPda};
use crate::errors::RegistryError;

#[derive(Accounts)]
pub struct UpdateProtocolConfig<'info> {
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK: authority is protocol config authority.
    #[account(mut, has_one=authority)]
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    /// CHECK: is signer to reduce risk of updating with a wrong authority.
    pub new_authority: Option<Signer<'info>>,
}

pub fn check_protocol_config(protocol_config: ProtocolConfig) -> Result<()> {
    if protocol_config.min_weight == 0 {
        msg!("Min weight cannot be zero.");
        return err!(RegistryError::InvalidConfigUpdate);
    }
    if protocol_config.active_phase_length < protocol_config.registration_phase_length {
        msg!(
            "Active phase length must be greater or equal than registration phase length. {} {}",
            protocol_config.active_phase_length,
            protocol_config.registration_phase_length
        );
        return err!(RegistryError::InvalidConfigUpdate);
    }
    if protocol_config.active_phase_length < protocol_config.report_work_phase_length {
        msg!(
            "Active phase length must be greater or equal than report work phase length. {} {}",
            protocol_config.active_phase_length,
            protocol_config.report_work_phase_length
        );
        return err!(RegistryError::InvalidConfigUpdate);
    }
    if protocol_config.active_phase_length < protocol_config.slot_length {
        msg!(
            "Active phase length is less than slot length, active phase length {} < slot length {}. (Active phase length must be greater than slot length.)",
            protocol_config.active_phase_length,
            protocol_config.slot_length
        );
        return err!(RegistryError::InvalidConfigUpdate);
    }
    Ok(())
}
