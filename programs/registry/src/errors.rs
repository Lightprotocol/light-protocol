use anchor_lang::prelude::*;

#[error_code]
pub enum RegistryError {
    #[msg("InvalidForester")]
    InvalidForester,
    NotInReportWorkPhase,
    StakeAccountAlreadySynced,
    EpochEnded,
    ForresterNotEligible,
    NotInRegistrationPeriod,
    StakeInsuffient,
    ForesterAlreadyRegistered,
    InvalidEpochAccount,
    InvalidEpoch,
    EpochStillInProgress,
    NotInActivePhase,
    ForesterAlreadyReportedWork,
    ComputeEscrowAmountFailed,
    InputEscrowTokenHashNotProvided,
    ArithmeticUnderflow,
    ArithmeticOverflow,
    AlreadyDelegated,
    InvalidAuthority,
    InvalidMint,
    HashToFieldError,
    StakeAccountSyncError,
    DepositAmountNotEqualInputAmount,
    InvalidProtocolConfigUpdate,
    DelegateAccountNotSynced,
}
