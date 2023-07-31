import * as anchor from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { relayerFee } from "../config";
import { confirmConfig, Provider, Relayer } from "@lightprotocol/zk.js";

import { readLookUpTableFromFile } from "./readLookUpTableFromFile";
require("dotenv").config();

let provider: Provider;
let relayer: Relayer;

export const getKeyPairFromEnv = (KEY: string) => {
  return Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(process.env[KEY] || "")),
  );
};

export const getAnchorProvider = async (): Promise<anchor.AnchorProvider> => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = process.env.RPC_URL;
  const url = process.env.RPC_URL;
  if (!url) throw new Error("Environment variable RPC_URL not set");
  const connection = new anchor.web3.Connection(url, "confirmed");
  const providerAnchor = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(getKeyPairFromEnv("KEY_PAIR")),
    confirmConfig,
  );
  return providerAnchor;
};

export const getLightProvider = async () => {
  console.log("@getLightProvider");
  if (!provider) {
    const relayer = await getRelayer();
    console.log("@getLightProvider relaye: ", relayer);
    try {
      provider = await Provider.init({
        wallet: getKeyPairFromEnv("KEY_PAIR"),
        relayer,
        confirmConfig,
        url: process.env.RPC_URL,
        versionedTransactionLookupTable: new PublicKey(
          readLookUpTableFromFile(),
        ),
      });
    } catch (e) {
      if (e.message.includes("LOOK_UP_TABLE_NOT_INITIALIZED")) {
        console.log("LOOK_UP_TABLE_NOT_INITIALIZED");
        // const anchorProvider = await setAnchorProvider();
        // console.log("ANCHOR PROVIDER set: ", anchorProvider);
        // let looupTable = await initLookUpTable(
        //   useWallet(getKeyPairFromEnv("KEY_PAIR")),
        //   anchorProvider,
        // );
        // console.log("INITED:", looupTable);
        // const replaceLookupTableValue = async (newValue: string) =>
        //   readFile(
        //     ".env",
        //     "utf8",
        //     (err, data) =>
        //       err ||
        //       writeFile(
        //         ".env",
        //         data.replace(/(LOOK_UP_TABLE=).*\n/, `$1${newValue}\n`),
        //         "utf8",
        //         console.error,
        //       ),
        //   );
        // process.env.LOOK_UP_TABLE = looupTable.toBase58();
        // await replaceLookupTableValue(looupTable.toBase58());
        // provider = await Provider.init({
        //   wallet: getKeyPairFromEnv("KEY_PAIR"),
        //   relayer,
        //   confirmConfig,
        //   url: process.env.RPC_URL,
        //   versionedTransactionLookupTable: looupTable,
        // });
        throw e;
      } else {
        throw e;
      }
    }
  }
  return provider;
};

export async function getRelayer() {
  if (!relayer) {
    relayer = new Relayer(
      getKeyPairFromEnv("KEY_PAIR").publicKey,
      new PublicKey(readLookUpTableFromFile()),
      getKeyPairFromEnv("RELAYER_RECIPIENT").publicKey,
      relayerFee,
    );

    return relayer;
  }
  return relayer;
}
