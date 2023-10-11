import { RELAYER_FEE, TOKEN_ACCOUNT_FEE } from "@lightprotocol/zk.js";

export const PSP_TEMPLATE_TAG = "v0.1.2";
export const PROGRAM_TAG = "v0.3.2";
export const MACRO_CIRCOM_TAG = "v0.1.6";
export const ZK_JS_VERSION = "0.3.2-alpha.15";
export const PROVER_JS_VERSION = "0.1.0-alpha.2";
export const CIRCUIT_LIB_CIRCOM_VERSION =
  "file:../circuit-lib/circuit-lib.circom"; //"0.1.0-alpha.1";
export const PSP_DEFAULT_PROGRAM_ID =
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";
export const LIGHT_SYSTEM_PROGRAMS_VERSION =
  '{ git = "https://github.com/lightprotocol/light-protocol", features = ["cpi"], branch = "jorrit/rename-verifier-two" }';
export const LIGHT_SYSTEM_PROGRAM = "light-psp4in4out-app-storage";
export const LIGHT_MACROS_VERSION =
  '{ git = "https://github.com/lightprotocol/light-protocol", branch = "jorrit/rename-verifier-two" }';
export const LIGHT_VERIFIER_SDK_VERSION =
  '{ git = "https://github.com/lightprotocol/light-protocol", branch = "jorrit/rename-verifier-two" }';
export const CONFIG_PATH = "/.config/light/";
export const CONFIG_FILE_NAME = "config.json";

export const DEFAULT_CONFIG = {
  relayerUrl: "http://localhost:3332",
  relayerRecipient: "AV3LnV78ezsEBZebNeMPtEcH1hmvSfUBC5Xbyrzqbt44",
  rpcUrl: "http://127.0.0.1:8899",
  relayerPublicKey: "EkXDLi1APzu6oxJbg5Hnjb24kfKauJp1xCb5FAUMxf9D",
  lookupTable: "8SezKuv7wMNPd574Sq4rQ1wvVrxa22xPYtkeruJRjrhG",
  relayerFee: RELAYER_FEE,
  highRelayerFee: TOKEN_ACCOUNT_FEE,
};
