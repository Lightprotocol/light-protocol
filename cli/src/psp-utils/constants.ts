import { RPC_FEE, TOKEN_ACCOUNT_FEE } from "@lightprotocol/zk.js";

export const SPL_NOOP_PROGRAM_TAG = "spl-noop-v0.2.0";
export const LIGHT_MERKLE_TREE_PROGRAM_TAG = "light-merkle-tree-program-v0.3.1";

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
