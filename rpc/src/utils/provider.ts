import * as anchor from "@coral-xyz/anchor";
import { Keypair } from "@solana/web3.js";
import { RPC_LOOK_UP_TABLE, SOLANA_RPC_URL, rpcFee } from "../config";
import {
  confirmConfig,
  Provider,
  Rpc,
  TOKEN_ACCOUNT_FEE,
  useWallet,
} from "@lightprotocol/zk.js";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";
import {
  EnvironmentVariableError,
  EnvironmentVariableErrorCode,
} from "../errors";
require("dotenv").config();

let provider: Provider;
let rpc: Rpc;

export const getKeyPairFromEnv = (KEY: string) => {
  return Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(process.env[KEY] || "")),
  );
};

export const getAnchorProvider = async (): Promise<anchor.AnchorProvider> => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = process.env.SOLANA_RPC_URL;
  const url = process.env.SOLANA_RPC_URL;
  if (!url)
    throw new EnvironmentVariableError(
      EnvironmentVariableErrorCode.VARIABLE_NOT_SET,
      "getAnchorProvider",
      "SOLANA_RPC_URL",
    );
  console.log("url", url);
  const connection = new anchor.web3.Connection(url, "confirmed");
  const providerAnchor = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(getKeyPairFromEnv("KEY_PAIR")),
    confirmConfig,
  );
  return providerAnchor;
};

export const getLightProvider = async () => {
  if (!provider) {
    const rpc = getRpc();

    try {
      const anchorProvider = await getAnchorProvider();
      const lightWasm: LightWasm = await WasmFactory.getInstance();

      provider = new Provider({
        lightWasm,
        wallet: useWallet(getKeyPairFromEnv("KEY_PAIR")),
        rpc,
        connection: anchorProvider.connection,
        url: process.env.SOLANA_RPC_URL!,
        versionedTransactionLookupTable: RPC_LOOK_UP_TABLE,
        anchorProvider,
      });
    } catch (e) {
      if (e.message.includes("LOOK_UP_TABLE_NOT_INITIALIZED")) {
        console.log("LOOK_UP_TABLE_NOT_INITIALIZED");
      } else {
        throw e;
      }
    }
  }
  return provider;
};

export function getRpc(): Rpc {
  if (!rpc) {
    rpc = new Rpc(
      getKeyPairFromEnv("KEY_PAIR").publicKey,
      getKeyPairFromEnv("RPC_RECIPIENT").publicKey,
      rpcFee,
      TOKEN_ACCOUNT_FEE,
      SOLANA_RPC_URL!,
    );

    return rpc;
  }
  return rpc;
}
