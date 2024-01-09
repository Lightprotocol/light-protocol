import { RPC_FEE, TOKEN_ACCOUNT_FEE } from "@lightprotocol/zk.js";

export const PSP_TEMPLATE_TAG = "v0.1.3";

export const SPL_NOOP_PROGRAM_TAG = "spl-noop-v0.2.0";
export const LIGHT_MERKLE_TREE_PROGRAM_TAG = "light-merkle-tree-program-v0.3.1";
export const LIGHT_PSP2IN2OUT_TAG = "light-psp2in2out-v0.3.1";
export const LIGHT_PSP2IN2OUT_STORAGE_TAG = "light-psp2in2out-storage-v0.3.1";
export const LIGHT_PSP4IN4OUT_APP_STORAGE_TAG =
  "light-psp4in4out-app-storage-v0.3.1";
export const LIGHT_PSP10IN2OUT_TAG = "light-psp10in2out-v0.3.1";
export const LIGHT_USER_REGISTRY_TAG = "light-user-registry-v0.3.0";

export const MACRO_CIRCOM_TAG = "macro-circom-v0.1.1";
export const ZK_JS_VERSION = "0.3.2-alpha.16";
export const PROVER_JS_VERSION = "0.1.0-alpha.3";
export const ACCOUNT_RS_VERSION = "0.0.1";
export const CIRCUIT_LIB_CIRCOM_VERSION = "0.1.0-alpha.1";
export const PSP_DEFAULT_PROGRAM_ID =
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

export const LIGHT_SYSTEM_PROGRAM = "light-psp4in4out-app-storage";
export const LIGHT_SYSTEM_PROGRAMS_VERSION =
  '{ git = "https://github.com/lightprotocol/light-protocol", features = ["cpi"], branch = "main" }';
export const LIGHT_MACROS_VERSION =
  '{ git = "https://github.com/lightprotocol/light-protocol", branch = "main" }';
export const LIGHT_VERIFIER_SDK_VERSION =
  '{ git = "https://github.com/lightprotocol/light-protocol", branch = "main" }';
export const CONFIG_PATH = "/.config/light/";
export const CONFIG_FILE_NAME = "config.json";

export const DEFAULT_CONFIG = {
  rpcUrl: "http://localhost:3332",
  rpcRecipient: "AV3LnV78ezsEBZebNeMPtEcH1hmvSfUBC5Xbyrzqbt44",
  solanaRpcUrl: "http://127.0.0.1:8899",
  rpcPublicKey: "EkXDLi1APzu6oxJbg5Hnjb24kfKauJp1xCb5FAUMxf9D",
  lookupTable: "8SezKuv7wMNPd574Sq4rQ1wvVrxa22xPYtkeruJRjrhG",
  rpcFee: RPC_FEE.toString(),
  highRpcFee: TOKEN_ACCOUNT_FEE.toString(),
};

// TODO: investigate why latest cargo-generate fails
// Fixed version because 11/11/23 release (v0.18.5) fails
export const CARGO_GENERATE_TAG = "v0.18.4";
