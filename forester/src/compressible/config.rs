use std::str::FromStr;

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

// =============================================================================
// Shared Constants
// =============================================================================

/// Registry program ID for compress_and_close operations
pub const REGISTRY_PROGRAM_ID: &str = "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX";

/// Offset in CToken/Mint account data where account_type byte is stored.
/// Used for memcmp filters to identify decompressed accounts.
pub const ACCOUNT_TYPE_OFFSET: usize = 165;

/// Base58-encoded byte value for decompressed CToken accounts (account_type = 2).
/// In base58: "3" represents the byte value 2.
pub const CTOKEN_ACCOUNT_TYPE_FILTER: &str = "3";

/// Base58-encoded byte value for decompressed Mint accounts (account_type = 1).
/// In base58: "2" represents the byte value 1.
pub const MINT_ACCOUNT_TYPE_FILTER: &str = "2";

/// Default page size for bootstrap pagination (number of accounts per RPC request)
pub const DEFAULT_PAGE_SIZE: usize = 10_000;

/// Default delay between paginated RPC requests (milliseconds)
pub const DEFAULT_PAGINATION_DELAY_MS: u64 = 100;

// =============================================================================
// Configuration Structs
// =============================================================================

/// Configuration for a compressible PDA program.
///
/// Can be specified via CLI (using `program_id:discriminator_hex` format)
/// or via config file using the serialized struct format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdaProgramConfig {
    /// Program ID that owns the compressible PDAs (base58 string)
    #[serde(with = "pubkey_string")]
    pub program_id: Pubkey,
    /// Discriminator for the compress_accounts_idempotent instruction (base58 string)
    #[serde(with = "discriminator_base58")]
    pub discriminator: [u8; 8],
}

mod pubkey_string {
    use std::str::FromStr;

    use serde::{Deserialize, Deserializer, Serializer};
    use solana_sdk::pubkey::Pubkey;

    pub fn serialize<S>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&pubkey.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Pubkey::from_str(&s).map_err(serde::de::Error::custom)
    }
}

mod discriminator_base58 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(discriminator: &[u8; 8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&bs58::encode(discriminator).into_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 8], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = bs58::decode(&s)
            .into_vec()
            .map_err(serde::de::Error::custom)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("discriminator must be exactly 8 bytes"))
    }
}

impl PdaProgramConfig {
    pub fn new(program_id: Pubkey, discriminator: [u8; 8]) -> Self {
        Self {
            program_id,
            discriminator,
        }
    }
}

impl FromStr for PdaProgramConfig {
    type Err = String;

    /// Parse from string format: "program_id:discriminator_base58"
    /// Example: "MyProgram1111111111111111111111111111111:6kRvHBv2N3F"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid format. Expected 'program_id:discriminator_base58', got '{}'",
                s
            ));
        }

        let program_id = parts[0]
            .parse::<Pubkey>()
            .map_err(|e| format!("Invalid program ID '{}': {}", parts[0], e))?;

        let disc_bytes = bs58::decode(parts[1])
            .into_vec()
            .map_err(|e| format!("Invalid discriminator base58 '{}': {}", parts[1], e))?;

        let disc_len = disc_bytes.len();
        let discriminator: [u8; 8] = disc_bytes
            .try_into()
            .map_err(|_| format!("Discriminator must be exactly 8 bytes, got {}", disc_len))?;

        Ok(Self {
            program_id,
            discriminator,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressibleConfig {
    /// WebSocket URL for account subscriptions
    pub ws_url: String,
    /// Batch size for compression operations
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// Maximum number of concurrent compression batches
    #[serde(default = "default_max_concurrent_batches")]
    pub max_concurrent_batches: usize,
    /// Compressible PDA programs to track and compress.
    /// Can be specified in config file or via CLI `--pda-program` flags.
    /// CLI values are merged with config file values.
    #[serde(default)]
    pub pda_programs: Vec<PdaProgramConfig>,
}

fn default_batch_size() -> usize {
    5
}

fn default_max_concurrent_batches() -> usize {
    10
}

impl CompressibleConfig {
    pub fn new(ws_url: String) -> Self {
        Self {
            ws_url,
            batch_size: default_batch_size(),
            max_concurrent_batches: default_max_concurrent_batches(),
            pda_programs: Vec::new(),
        }
    }

    pub fn with_pda_programs(mut self, pda_programs: Vec<PdaProgramConfig>) -> Self {
        self.pda_programs = pda_programs;
        self
    }
}
