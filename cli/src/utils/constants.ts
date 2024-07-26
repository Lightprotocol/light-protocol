export const SPL_NOOP_PROGRAM_TAG = "spl-noop-v0.2.0";
export const LIGHT_MERKLE_TREE_PROGRAM_TAG = "light-merkle-tree-program-v0.3.1";
export const LIGHT_ACCOUNT_COMPRESSION_TAG = "account-compression-v0.3.5";
export const LIGHT_SYSTEM_PROGRAM_TAG = "light-system-program-v0.3.4";
export const LIGHT_REGISTRY_TAG = "light-registry-v0.3.4";
export const LIGHT_COMPRESSED_TOKEN_TAG = "light-compressed-token-v0.3.4";

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
export const FORESTER_PROCESS_NAME = "forester";

export const PHOTON_VERSION = "0.38.0";

export const LIGHT_PROTOCOL_PROGRAMS_DIR_ENV = "LIGHT_PROTOCOL_PROGRAMS_DIR";
export const BASE_PATH = "../../bin/";
