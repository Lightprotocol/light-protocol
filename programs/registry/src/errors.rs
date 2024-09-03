use anchor_lang::prelude::*;

#[error_code]
pub enum RegistryError {
    #[msg("InvalidForester")]
    InvalidForester,
    NotInReportWorkPhase,
    StakeAccountAlreadySynced,
    EpochEnded,
    ForesterNotEligible,
    NotInRegistrationPeriod,
    WeightInsuffient,
    ForesterAlreadyRegistered,
    InvalidEpochAccount,
    InvalidEpoch,
    EpochStillInProgress,
    NotInActivePhase,
    ForesterAlreadyReportedWork,
    InvalidNetworkFee,
    FinalizeCounterExceeded,
    CpiContextAccountMissing,
    ArithmeticUnderflow,
    RegistrationNotFinalized,
    CpiContextAccountInvalidDataLen,
    InvalidConfigUpdate,
    InvalidSigner,
    GetLatestRegisterEpochFailed,
    GetCurrentActiveEpochFailed,
    ForesterUndefined,
    ForesterDefined,
}
