export const SPL_NOOP_PROGRAM_TAG = "spl-noop-v0.2.0";
export const LIGHT_ACCOUNT_COMPRESSION_TAG = "account-compression-v1.0.0";
export const LIGHT_SYSTEM_PROGRAM_TAG = "light-system-program-v1.0.0";
export const LIGHT_REGISTRY_TAG = "light-registry-v1.0.0";
export const LIGHT_COMPRESSED_TOKEN_TAG = "light-compressed-token-v1.0.0";

export const CONFIG_PATH = "/.config/light/";
export const CONFIG_FILE_NAME = "config.json";

export const DEFAULT_CONFIG = {
  solanaRpcUrl: "http://127.0.0.1:8899",
};

// TODO: investigate why latest cargo-generate fails
// Fixed version because 11/11/23 release (v0.18.5) fails
export const CARGO_GENERATE_TAG = "v0.18.4";

export const SOLANA_VALIDATOR_PROCESS_NAME = "solana-test-validator";
export const LIGHT_PROVER_PROCESS_NAME = "light-prover";
export const INDEXER_PROCESS_NAME = "photon";

export const PHOTON_VERSION = "0.50.0";

export const LIGHT_PROTOCOL_PROGRAMS_DIR_ENV = "LIGHT_PROTOCOL_PROGRAMS_DIR";
export const BASE_PATH = "../../bin/";

export const PROGRAM_ID = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";
export const SOLANA_SDK_VERSION = "2.2.1";
export const ANCHOR_VERSION = "0.31.1";
export const BORSH_VERSION = "0.10.4";
export const COMPRESSED_PROGRAM_TEMPLATE_TAG = "v0.1.2";
export const TOKIO_VERSION = "1.36.0";
export const SOLANA_PROGRAM_TEST_VERSION = "2.2.1";

export const LIGHT_HASHER_VERSION = "2.0.0";
export const LIGHT_MACROS_VERSION = "1.1.0";
export const LIGHT_SDK_VERSION = "0.10.0";
export const LIGHT_COMPRESSED_ACCOUNT_VERSION = "0.1.0";
export const LIGHT_VERIFIER_VERSION = "1.1.0";
export const LIGHT_CLIENT_VERSION = "0.9.1";
// TODO: replace with light program test
export const LIGHT_TEST_UTILS_VERSION = "1.2.1";
