use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature,
    transaction::TransactionError,
};
use tabled::{builder::Builder, settings::Style};

const SYSTEM_PROGRAM_ID: &str = "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7";
const REGISTRY_PROGRAM_ID: &str = "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX";
const VERIFIER_PROGRAM_ID: &str = "VerYwRea726oghZ2EYaQt1N6bRzS2VnAveCwEcy6okj";
const FORESTER_PUBKEYS: &[&str] = &[
    "8GDc4p3fpbxJZmpZB3Lx3yN1984XS2HVnMi7J7rTyeC7",
    "3PrXqmhEcgPo2a5aTtCTYzgmuXRSx5imbUTDkz6SZMun",
];

#[derive(Debug, Parser)]
pub struct Options {
    /// One or more base58 public keys (positional, space-separated)
    pubkeys: Vec<String>,

    /// Bucket size in minutes
    #[clap(long, default_value_t = 10)]
    minutes: u64,

    /// Number of buckets to show going back in time
    #[clap(long, default_value_t = 3)]
    buckets: u64,

    /// Network: mainnet | devnet | testnet | local
    #[clap(long, default_value = "mainnet")]
    network: String,

    /// Custom RPC URL (overrides --network and SOLANA_RPC_URL)
    #[clap(long)]
    rpc_url: Option<String>,

    /// Print per-bucket error type breakdown with resolved names
    #[clap(long, short)]
    verbose: bool,

    /// Include Light System program (SySTEM1e...)
    #[clap(long)]
    system: bool,

    /// Include Light Registry program (Lighton6...)
    #[clap(long)]
    registry: bool,

    /// Include Verifier program (VerYwRea...)
    #[clap(long)]
    verifier: bool,

    /// Include both known forester keypairs
    #[clap(long)]
    forester: bool,

    /// Short mode: 1-minute buckets, 10 buckets (overrides --minutes and --buckets)
    #[clap(long, short)]
    short: bool,
}

fn network_to_url(network: &str) -> String {
    match network {
        "mainnet" => "https://api.mainnet-beta.solana.com".to_string(),
        "devnet" => "https://api.devnet.solana.com".to_string(),
        "testnet" => "https://api.testnet.solana.com".to_string(),
        "local" | "localnet" => "http://localhost:8899".to_string(),
        custom => custom.to_string(),
    }
}

fn shorten_address(addr: &str) -> String {
    if addr.len() > 10 {
        format!("{}...{}", &addr[..6], &addr[addr.len() - 3..])
    } else {
        addr.to_string()
    }
}

// ---------------------------------------------------------------------------
// Error annotation
//
// All main Light programs (system, account-compression, registry) use Anchor's
// #[error_code] which starts at offset 6000, so codes in that range are
// ambiguous across programs.  We show all matching candidates.
//
// Libraries use unique non-overlapping ranges:
//   7001-7012  : HasherError
//  10001-10014 : ConcurrentMerkleTreeError
//  11001-11009 : IndexedMerkleTreeError
//  14301-14312 : BatchedMerkleTreeError
//  15001-15017 : ZeroCopyError
//  16001-16050 : LightSdkError
//  20000-20017 : AccountError (account-checks)
// ---------------------------------------------------------------------------

/// Canonical key for an error: strips the instruction index so the same error
/// code from different instruction positions groups together.
fn error_key(err: &TransactionError) -> String {
    match err {
        TransactionError::InstructionError(_, inner) => format!("{:?}", inner),
        other => format!("{:?}", other),
    }
}

/// Parse the N out of a `Custom(N)` string.
fn parse_custom_code(key: &str) -> Option<u32> {
    let prefix = "Custom(";
    let start = key.find(prefix)? + prefix.len();
    let end = key[start..].find(')')? + start;
    key[start..end].parse().ok()
}

/// Annotate an error key with human-readable program::ErrorName candidates.
fn annotate_key(key: &str) -> String {
    if let Some(code) = parse_custom_code(key) {
        let candidates = lookup_error_names(code);
        if candidates.is_empty() {
            key.to_string()
        } else {
            format!("Custom({}) [{}]", code, candidates.join(" | "))
        }
    } else {
        key.to_string()
    }
}

fn lookup_error_names(code: u32) -> Vec<&'static str> {
    if let Some(name) = lookup_unique_range(code) {
        return vec![name];
    }
    let mut out = Vec::new();
    if let Some(n) = lookup_system_error(code) {
        out.push(n);
    }
    if let Some(n) = lookup_acc_compression_error(code) {
        out.push(n);
    }
    if let Some(n) = lookup_registry_error(code) {
        out.push(n);
    }
    out
}

fn lookup_unique_range(code: u32) -> Option<&'static str> {
    match code {
        // HasherError (7001-7012)
        7001 => Some("hasher::IntegerOverflow"),
        7002 => Some("hasher::Poseidon"),
        7003 => Some("hasher::PoseidonSyscall"),
        7004 => Some("hasher::UnknownSolanaSyscall"),
        7005 => Some("hasher::InvalidInputLength"),
        7006 => Some("hasher::InvalidNumFields"),
        7007 => Some("hasher::EmptyInput"),
        7008 => Some("hasher::BorshError"),
        7009 => Some("hasher::OptionHashToFieldSizeZero"),
        7010 => Some("hasher::PoseidonFeatureNotEnabled"),
        7011 => Some("hasher::Sha256FeatureNotEnabled"),
        7012 => Some("hasher::KeccakFeatureNotEnabled"),
        // ConcurrentMerkleTreeError (10001-10014)
        10001 => Some("concurrent-mt::IntegerOverflow"),
        10002 => Some("concurrent-mt::HeightZero"),
        10003 => Some("concurrent-mt::InvalidHeight"),
        10004 => Some("concurrent-mt::ChangelogZero"),
        10005 => Some("concurrent-mt::RootsZero"),
        10006 => Some("concurrent-mt::CanopyGeThanHeight"),
        10007 => Some("concurrent-mt::TreeIsFull"),
        10008 => Some("concurrent-mt::BatchGreaterThanChangelog"),
        10009 => Some("concurrent-mt::InvalidProofLength"),
        10010 => Some("concurrent-mt::InvalidProof"),
        10011 => Some("concurrent-mt::CannotUpdateLeaf"),
        10012 => Some("concurrent-mt::CannotUpdateEmpty"),
        10013 => Some("concurrent-mt::EmptyLeaves"),
        10014 => Some("concurrent-mt::BufferSize"),
        // IndexedMerkleTreeError (11001-11009)
        11001 => Some("indexed-mt::IntegerOverflow"),
        11002 => Some("indexed-mt::IndexHigherThanMax"),
        11003 => Some("indexed-mt::LowElementNotFound"),
        11004 => Some("indexed-mt::LowElementGreaterOrEqualToNewElement"),
        11005 => Some("indexed-mt::NewElementGreaterOrEqualToNextElement"),
        11006 => Some("indexed-mt::ElementAlreadyExists"),
        11007 => Some("indexed-mt::ElementDoesNotExist"),
        11008 => Some("indexed-mt::ChangelogBufferSize"),
        11009 => Some("indexed-mt::ArrayFull"),
        // BatchedMerkleTreeError (14301-14312)
        14301 => Some("batched-mt::BatchNotReady"),
        14302 => Some("batched-mt::BatchAlreadyInserted"),
        14303 => Some("batched-mt::BatchInsertFailed"),
        14304 => Some("batched-mt::LeafIndexNotInBatch"),
        14305 => Some("batched-mt::InvalidNetworkFee"),
        14306 => Some("batched-mt::BatchSizeNotDivisibleByZkpBatchSize"),
        14307 => Some("batched-mt::InclusionProofByIndexFailed"),
        14308 => Some("batched-mt::InvalidBatchIndex"),
        14309 => Some("batched-mt::InvalidIndex"),
        14310 => Some("batched-mt::TreeIsFull"),
        14311 => Some("batched-mt::NonInclusionCheckFailed"),
        14312 => Some("batched-mt::BloomFilterNotZeroed"),
        // ZeroCopyError (15001-15017, note: 15005 unused)
        15001 => Some("zero-copy::Full"),
        15002 => Some("zero-copy::ArraySize"),
        15003 => Some("zero-copy::IterFromOutOfBounds"),
        15004 => Some("zero-copy::InsufficientMemoryAllocated"),
        15006 => Some("zero-copy::UnalignedPointer"),
        15007 => Some("zero-copy::MemoryNotZeroed"),
        15008 => Some("zero-copy::InvalidConversion"),
        15009 => Some("zero-copy::InvalidData"),
        15010 => Some("zero-copy::Size"),
        15011 => Some("zero-copy::InvalidOptionByte"),
        15012 => Some("zero-copy::InvalidCapacity"),
        15013 => Some("zero-copy::LengthGreaterThanCapacity"),
        15014 => Some("zero-copy::CurrentIndexGreaterThanLength"),
        15015 => Some("zero-copy::InvalidEnumValue"),
        15016 => Some("zero-copy::InsufficientCapacity"),
        15017 => Some("zero-copy::PlatformSizeOverflow"),
        // LightSdkError (16001-16050)
        16001 => Some("sdk::ConstraintViolation"),
        16002 => Some("sdk::InvalidLightSystemProgram"),
        16003 => Some("sdk::ExpectedAccounts"),
        16004 => Some("sdk::ExpectedAddressTreeInfo"),
        16005 => Some("sdk::ExpectedAddressRootIndex"),
        16006 => Some("sdk::ExpectedData"),
        16007 => Some("sdk::ExpectedDiscriminator"),
        16008 => Some("sdk::ExpectedHash"),
        16009 => Some("sdk::ExpectedLightSystemAccount"),
        16010 => Some("sdk::ExpectedMerkleContext"),
        16011 => Some("sdk::ExpectedRootIndex"),
        16012 => Some("sdk::TransferFromNoInput"),
        16013 => Some("sdk::TransferFromNoLamports"),
        16014 => Some("sdk::TransferFromInsufficientLamports"),
        16015 => Some("sdk::TransferIntegerOverflow"),
        16016 => Some("sdk::Borsh"),
        16017 => Some("sdk::FewerAccountsThanSystemAccounts"),
        16018 => Some("sdk::InvalidCpiSignerAccount"),
        16019 => Some("sdk::MissingField"),
        16020 => Some("sdk::OutputStateTreeIndexIsNone"),
        16021 => Some("sdk::InitAddressIsNone"),
        16022 => Some("sdk::InitWithAddressIsNone"),
        16023 => Some("sdk::InitWithAddressOutputIsNone"),
        16024 => Some("sdk::MetaMutAddressIsNone"),
        16025 => Some("sdk::MetaMutInputIsNone"),
        16026 => Some("sdk::MetaMutOutputLamportsIsNone"),
        16027 => Some("sdk::MetaMutOutputIsNone"),
        16028 => Some("sdk::MetaCloseAddressIsNone"),
        16029 => Some("sdk::MetaCloseInputIsNone"),
        16031 => Some("sdk::CpiAccountsIndexOutOfBounds"),
        16032 => Some("sdk::InvalidCpiContextAccount"),
        16033 => Some("sdk::InvalidSolPoolPdaAccount"),
        16034 => Some("sdk::InvalidCpiAccountsOffset"),
        16035 => Some("sdk::ExpectedNoData"),
        16036 => Some("sdk::CpiContextOrderingViolation"),
        16037 => Some("sdk::InvalidMerkleTreeIndex"),
        16038 => Some("sdk::ReadOnlyAccountCannotUseToAccountInfo"),
        16039 => Some("sdk::NotReadOnlyAccount"),
        16040 => Some("sdk::ReadOnlyAccountsNotSupportedInCpiContext"),
        16041 => Some("sdk::ExpectedTreeInfo"),
        16042 => Some("sdk::ExpectedSelfProgram"),
        16043 => Some("sdk::ExpectedCpiContext"),
        16044 => Some("sdk::MissingCompressionInfo"),
        16045 => Some("sdk::PackedVariantCompressionInfo"),
        16046 => Some("sdk::CTokenCompressionInfo"),
        16047 => Some("sdk::UnexpectedUnpackedVariant"),
        16048 => Some("sdk::TokenPrepareCalled"),
        16049 => Some("sdk::ZeroCopyUnpackedVariant"),
        16050 => Some("sdk::InvalidRentSponsor"),
        // AccountError / account-checks (20000-20017)
        20000 => Some("account-checks::InvalidDiscriminator"),
        20001 => Some("account-checks::AccountOwnedByWrongProgram"),
        20002 => Some("account-checks::AccountNotMutable"),
        20003 => Some("account-checks::BorrowAccountDataFailed"),
        20004 => Some("account-checks::InvalidAccountSize"),
        20005 => Some("account-checks::AccountMutable"),
        20006 => Some("account-checks::AlreadyInitialized"),
        20007 => Some("account-checks::InvalidAccountBalance"),
        20008 => Some("account-checks::FailedBorrowRentSysvar"),
        20009 => Some("account-checks::InvalidSigner"),
        20010 => Some("account-checks::InvalidSeeds"),
        20011 => Some("account-checks::InvalidProgramId"),
        20012 => Some("account-checks::ProgramNotExecutable"),
        20013 => Some("account-checks::AccountNotZeroed"),
        20014 => Some("account-checks::NotEnoughAccountKeys"),
        20015 => Some("account-checks::InvalidAccount"),
        20016 => Some("account-checks::FailedSysvarAccess"),
        20017 => Some("account-checks::ArithmeticOverflow"),
        _ => None,
    }
}

fn lookup_system_error(code: u32) -> Option<&'static str> {
    match code {
        6000 => Some("system::SumCheckFailed"),
        6001 => Some("system::SignerCheckFailed"),
        6002 => Some("system::CpiSignerCheckFailed"),
        6003 => Some("system::ComputeInputSumFailed"),
        6004 => Some("system::ComputeOutputSumFailed"),
        6005 => Some("system::ComputeRpcSumFailed"),
        6006 => Some("system::InvalidAddress"),
        6007 => Some("system::DeriveAddressError"),
        6008 => Some("system::CompressedSolPdaUndefinedForCompressSol"),
        6009 => Some("system::DecompressLamportsUndefinedForCompressSol"),
        6010 => Some("system::CompressedSolPdaUndefinedForDecompressSol"),
        6011 => Some("system::DeCompressLamportsUndefinedForDecompressSol"),
        6012 => Some("system::DecompressRecipientUndefinedForDecompressSol"),
        6013 => Some("system::WriteAccessCheckFailed"),
        6014 => Some("system::InvokingProgramNotProvided"),
        6015 => Some("system::InvalidCapacity"),
        6016 => Some("system::InvalidMerkleTreeOwner"),
        6017 => Some("system::ProofIsNone"),
        6018 => Some("system::ProofIsSome"),
        6019 => Some("system::EmptyInputs"),
        6020 => Some("system::CpiContextAccountUndefined"),
        6021 => Some("system::CpiContextEmpty"),
        6022 => Some("system::CpiContextMissing"),
        6023 => Some("system::DecompressionRecipientDefined"),
        6024 => Some("system::SolPoolPdaDefined"),
        6025 => Some("system::AppendStateFailed"),
        6026 => Some("system::InstructionNotCallable"),
        6027 => Some("system::CpiContextFeePayerMismatch"),
        6028 => Some("system::CpiContextAssociatedMerkleTreeMismatch"),
        6029 => Some("system::NoInputs"),
        6030 => Some("system::InputMerkleTreeIndicesNotInOrder"),
        6031 => Some("system::OutputMerkleTreeIndicesNotInOrder"),
        6032 => Some("system::OutputMerkleTreeNotUnique"),
        6033 => Some("system::DataFieldUndefined"),
        6034 => Some("system::ReadOnlyAddressAlreadyExists"),
        6035 => Some("system::ReadOnlyAccountDoesNotExist"),
        6036 => Some("system::HashChainInputsLenghtInconsistent"),
        6037 => Some("system::InvalidAddressTreeHeight"),
        6038 => Some("system::InvalidStateTreeHeight"),
        6039 => Some("system::InvalidArgument"),
        6040 => Some("system::InvalidAccount"),
        6041 => Some("system::AddressMerkleTreeAccountDiscriminatorMismatch"),
        6042 => Some("system::StateMerkleTreeAccountDiscriminatorMismatch"),
        6043 => Some("system::ProofVerificationFailed"),
        6044 => Some("system::InvalidAccountMode"),
        6045 => Some("system::InvalidInstructionDataDiscriminator"),
        6046 => Some("system::NewAddressAssignedIndexOutOfBounds"),
        6047 => Some("system::AddressIsNone"),
        6048 => Some("system::AddressDoesNotMatch"),
        6049 => Some("system::CpiContextAlreadySet"),
        6050 => Some("system::InvalidTreeHeight"),
        6051 => Some("system::TooManyOutputAccounts"),
        6052 => Some("system::BorrowingDataFailed"),
        6053 => Some("system::DuplicateAccountInInputsAndReadOnly"),
        6054 => Some("system::CpiContextPassedAsSetContext"),
        6055 => Some("system::InvalidCpiContextOwner"),
        6056 => Some("system::InvalidCpiContextDiscriminator"),
        6057 => Some("system::InvalidAccountIndex"),
        6058 => Some("system::AccountCompressionCpiDataExceedsLimit"),
        6059 => Some("system::AddressOwnerIndexOutOfBounds"),
        6060 => Some("system::AddressAssignedAccountIndexOutOfBounds"),
        6061 => Some("system::OutputMerkleTreeIndexOutOfBounds"),
        6062 => Some("system::PackedAccountIndexOutOfBounds"),
        6063 => Some("system::Unimplemented"),
        6064 => Some("system::CpiContextDeactivated"),
        6065 => Some("system::InputMerkleTreeIndexOutOfBounds"),
        6066 => Some("system::MissingLegacyMerkleContext"),
        _ => None,
    }
}

fn lookup_acc_compression_error(code: u32) -> Option<&'static str> {
    match code {
        6000 => Some("acc-compr::AddressMerkleTreeAccountDiscriminatorMismatch"),
        6001 => Some("acc-compr::EmptyLeaf"),
        6002 => Some("acc-compr::InputDeserializationFailed"),
        6003 => Some("acc-compr::InputElementsEmpty"),
        6004 => Some("acc-compr::InsufficientRolloverFee"),
        6005 => Some("acc-compr::IntegerOverflow"),
        6006 => Some("acc-compr::InvalidAccount"),
        6007 => Some("acc-compr::InvalidAccountBalance"),
        6008 => Some("acc-compr::InvalidAccountSize"),
        6009 => Some("acc-compr::InvalidAuthority"),
        6010 => Some("acc-compr::InvalidGroup"),
        6011 => Some("acc-compr::InvalidMerkleProof"),
        6012 => Some("acc-compr::InvalidNoopPubkey"),
        6013 => Some("acc-compr::InvalidQueueType"),
        6014 => Some("acc-compr::InvalidSequenceThreshold"),
        6015 => Some("acc-compr::LeafNotFound"),
        6016 => Some("acc-compr::MerkleTreeAlreadyRolledOver"),
        6017 => Some("acc-compr::MerkleTreeAndQueueNotAssociated"),
        6018 => Some("acc-compr::NoLeavesForMerkleTree"),
        6019 => Some("acc-compr::NotAllLeavesProcessed"),
        6020 => Some("acc-compr::NotReadyForRollover"),
        6021 => Some("acc-compr::NumberOfChangeLogIndicesMismatch"),
        6022 => Some("acc-compr::NumberOfIndicesMismatch"),
        6023 => Some("acc-compr::NumberOfLeavesMismatch"),
        6024 => Some("acc-compr::NumberOfProofsMismatch"),
        6025 => Some("acc-compr::ProofLengthMismatch"),
        6026 => Some("acc-compr::RegistryProgramIsNone"),
        6027 => Some("acc-compr::RolloverNotConfigured"),
        6028 => Some("acc-compr::StateMerkleTreeAccountDiscriminatorMismatch"),
        6029 => Some("acc-compr::TooManyLeaves"),
        6030 => Some("acc-compr::TxHashUndefined"),
        6031 => Some("acc-compr::UnsupportedAdditionalBytes"),
        6032 => Some("acc-compr::UnsupportedCanopyDepth"),
        6033 => Some("acc-compr::UnsupportedCloseThreshold"),
        6034 => Some("acc-compr::UnsupportedHeight"),
        6035 => Some("acc-compr::UnsupportedParameters"),
        6036 => Some("acc-compr::V1AccountMarkedAsProofByIndex"),
        6037 => Some("acc-compr::TooManyAddresses"),
        6038 => Some("acc-compr::TooManyNullifiers"),
        _ => None,
    }
}

fn lookup_registry_error(code: u32) -> Option<&'static str> {
    match code {
        6000 => Some("registry::InvalidForester"),
        6001 => Some("registry::NotInReportWorkPhase"),
        6002 => Some("registry::StakeAccountAlreadySynced"),
        6003 => Some("registry::EpochEnded"),
        6004 => Some("registry::ForesterNotEligible"),
        6005 => Some("registry::NotInRegistrationPeriod"),
        6006 => Some("registry::WeightInsuffient"),
        6007 => Some("registry::ForesterAlreadyRegistered"),
        6008 => Some("registry::InvalidEpochAccount"),
        6009 => Some("registry::InvalidEpoch"),
        6010 => Some("registry::EpochStillInProgress"),
        6011 => Some("registry::NotInActivePhase"),
        6012 => Some("registry::ForesterAlreadyReportedWork"),
        6013 => Some("registry::InvalidNetworkFee"),
        6014 => Some("registry::FinalizeCounterExceeded"),
        6015 => Some("registry::CpiContextAccountMissing"),
        6016 => Some("registry::ArithmeticUnderflow"),
        6017 => Some("registry::RegistrationNotFinalized"),
        6018 => Some("registry::CpiContextAccountInvalidDataLen"),
        6019 => Some("registry::InvalidConfigUpdate"),
        6020 => Some("registry::InvalidSigner"),
        6021 => Some("registry::GetLatestRegisterEpochFailed"),
        6022 => Some("registry::GetCurrentActiveEpochFailed"),
        6023 => Some("registry::ForesterUndefined"),
        6024 => Some("registry::ForesterDefined"),
        6025 => Some("registry::InsufficientFunds"),
        6026 => Some("registry::ProgramOwnerDefined"),
        6027 => Some("registry::ProgramOwnerUndefined"),
        6028 => Some("registry::InvalidConfigState"),
        6029 => Some("registry::InvalidTokenAccountData"),
        6030 => Some("registry::EmptyIndices"),
        6031 => Some("registry::BorrowAccountDataFailed"),
        6032 => Some("registry::SerializationFailed"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------

pub async fn fetch_keypair_txs(opts: Options) -> Result<()> {
    let (minutes, buckets) = if opts.short {
        (1, 10)
    } else {
        (opts.minutes, opts.buckets)
    };

    // Expand pubkeys from preset flags (additive with positional args).
    let mut pubkeys = opts.pubkeys.clone();
    if opts.system {
        pubkeys.push(SYSTEM_PROGRAM_ID.to_string());
    }
    if opts.registry {
        pubkeys.push(REGISTRY_PROGRAM_ID.to_string());
    }
    if opts.verifier {
        pubkeys.push(VERIFIER_PROGRAM_ID.to_string());
    }
    if opts.forester {
        pubkeys.extend(FORESTER_PUBKEYS.iter().map(|s| s.to_string()));
    }
    if pubkeys.is_empty() {
        anyhow::bail!(
            "no addresses specified — provide pubkeys as positional args or use --system / --registry / --verifier / --forester"
        );
    }

    let rpc_url = opts.rpc_url.unwrap_or_else(|| {
        std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| network_to_url(&opts.network))
    });

    println!(
        "Fetching transactions for {} address(es)  |  bucket: {} min  |  looking back {} buckets",
        pubkeys.len(),
        minutes,
        buckets
    );
    let display_url = if let Some(idx) = rpc_url.find("api-key=") {
        format!("{}api-key=***", &rpc_url[..idx])
    } else {
        rpc_url.clone()
    };
    println!("RPC: {}", display_url);
    println!();

    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let now = Utc::now().timestamp();
    let bucket_secs = minutes as i64 * 60;
    let total_lookback = bucket_secs * buckets as i64;
    let cutoff = now - total_lookback;

    // Build header: Address | -10m ok | fail | TPS | -20m ok | fail | TPS | ... | Total | Fail
    let mut header = vec!["Address".to_string()];
    for k in 1..=buckets {
        header.push(format!("-{}m ok", k * minutes));
        header.push("fail".to_string());
        header.push("TPS".to_string());
    }
    header.push("Total".to_string());
    header.push("Fail".to_string());

    let mut builder = Builder::default();
    builder.push_record(header);

    // Per-pubkey verbose data: (pubkey, per-bucket error-key -> count)
    let mut verbose_data: Vec<(String, Vec<HashMap<String, u64>>)> = Vec::new();

    for pubkey_str in &pubkeys {
        let pubkey = Pubkey::from_str(pubkey_str)?;

        let mut ok_counts = vec![0u64; buckets as usize];
        let mut fail_counts = vec![0u64; buckets as usize];
        let mut error_maps: Vec<HashMap<String, u64>> = if opts.verbose {
            (0..buckets as usize).map(|_| HashMap::new()).collect()
        } else {
            Vec::new()
        };
        let mut before: Option<Signature> = None;

        loop {
            let batch = client.get_signatures_for_address_with_config(
                &pubkey,
                GetConfirmedSignaturesForAddress2Config {
                    before,
                    limit: Some(1000),
                    commitment: Some(CommitmentConfig::confirmed()),
                    ..Default::default()
                },
            )?;

            let exhausted = batch.len() < 1000;
            let mut reached_cutoff = false;

            for sig_info in &batch {
                if let Some(block_time) = sig_info.block_time {
                    if block_time < cutoff {
                        reached_cutoff = true;
                        continue;
                    }
                    let age = now - block_time;
                    let bucket_idx = if age < 0 {
                        0
                    } else {
                        (age / bucket_secs) as usize
                    };
                    if bucket_idx < buckets as usize {
                        match &sig_info.err {
                            None => ok_counts[bucket_idx] += 1,
                            Some(err) => {
                                fail_counts[bucket_idx] += 1;
                                if opts.verbose {
                                    let key = error_key(err);
                                    *error_maps[bucket_idx].entry(key).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }

            if exhausted || reached_cutoff {
                break;
            }

            match batch.last() {
                Some(last) => before = Some(Signature::from_str(&last.signature)?),
                None => break,
            }
        }

        let total_ok: u64 = ok_counts.iter().sum();
        let total_fail: u64 = fail_counts.iter().sum();

        let mut row = vec![shorten_address(pubkey_str)];
        for (ok, fail) in ok_counts.iter().zip(fail_counts.iter()) {
            let total = ok + fail;
            row.push(ok.to_string());
            row.push(fail.to_string());
            let tps = total as f64 / bucket_secs as f64;
            row.push(format!("{:.3}", tps));
        }
        row.push((total_ok + total_fail).to_string());
        row.push(total_fail.to_string());

        builder.push_record(row);

        if opts.verbose {
            verbose_data.push((pubkey_str.clone(), error_maps));
        }
    }

    let table = builder.build().with(Style::rounded()).to_string();
    println!("{}", table);

    if opts.verbose {
        for (pubkey_str, error_maps) in &verbose_data {
            println!("\n{}", pubkey_str);
            for (k, errors) in error_maps.iter().enumerate() {
                if errors.is_empty() {
                    continue;
                }
                let label = format!("-{}m", (k + 1) as u64 * minutes);
                let mut sorted: Vec<_> = errors.iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(a.1));
                println!("  {}:", label);
                for (key, count) in sorted {
                    println!("    {:>5}  {}", count, annotate_key(key));
                }
            }
        }
    }

    Ok(())
}
