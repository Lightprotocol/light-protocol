import * as anchor from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { relayerFee, rpcPort } from "../config";
import { confirmConfig, initLookUpTable, Provider, Relayer, useWallet } from "@lightprotocol/zk.js";
import { readFile, writeFile } from "fs";
import { tryCatch } from "bullmq";
require("dotenv").config();

let provider: Provider;
let relayer: Relayer;

export const getKeyPairFromEnv = (KEY: string) => {
  return Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(process.env[KEY] || "")),
  );
};

export const setAnchorProvider = async (): Promise<anchor.AnchorProvider> => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = `http://127.0.0.1:${rpcPort}`; // runscript starts dedicated validator on this port.

  const providerAnchor = anchor.AnchorProvider.local(
    `http://127.0.0.1:${rpcPort}`,
    confirmConfig,
  );

  anchor.setProvider(providerAnchor);

  return providerAnchor;
};

export const getLightProvider = async () => {
  if (!provider) {
    const relayer = await getRelayer();
    try {
      
    provider = await Provider.init({
      wallet: getKeyPairFromEnv("KEY_PAIR"),
      relayer,
      confirmConfig,
      url: process.env.RPC_URL,
      versionedTransactionLookupTable: new PublicKey(process.env.LOOK_UP_TABLE!),
    });
  } catch (e) {
    // console.log("LOOK_UP_TABLE_NOT_INITIALIZED")
    // const anchorProvider = await setAnchorProvider();
    //     let looupTable = await initLookUpTable(useWallet(getKeyPairFromEnv("KEY_PAIR")),anchorProvider );
    //   const replaceLookupTableValue = async (newValue: string) =>
    //     readFile('.env', 'utf8', (err, data) =>
    //       err || writeFile('.env', data.replace(/(LOOK_UP_TABLE=).*\n/, `$1${newValue}\n`), 'utf8', console.error)
    //     );
    //     console.log("looupTable",  looupTable)
    //   process.env.LOOK_UP_TABLE = looupTable.toBase58();
    //   await replaceLookupTableValue(looupTable.toBase58());
    //   provider = await Provider.init({
    //     wallet: getKeyPairFromEnv("KEY_PAIR"),
    //     relayer,
    //     confirmConfig,
    //     url: process.env.RPC_URL,
    //     versionedTransactionLookupTable: looupTable,
    //   });
      if(e.message.includes("LOOK_UP_TABLE_NOT_INITIALIZED")) {
      const anchorProvider = await setAnchorProvider();
          let looupTable = await initLookUpTable(useWallet(getKeyPairFromEnv("KEY_PAIR")),anchorProvider );
        const replaceLookupTableValue = async (newValue: string) =>
          readFile('.env', 'utf8', (err, data) =>
            err || writeFile('.env', data.replace(/(LOOK_UP_TABLE=).*\n/, `$1${newValue}\n`), 'utf8', console.error)
          );
        process.env.LOOK_UP_TABLE = looupTable.toBase58();
        await replaceLookupTableValue(looupTable.toBase58());
        provider = await Provider.init({
          wallet: getKeyPairFromEnv("KEY_PAIR"),
          relayer,
          confirmConfig,
          url: process.env.RPC_URL,
          versionedTransactionLookupTable: looupTable,
        });
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
      new PublicKey(process.env.LOOK_UP_TABLE!),
      getKeyPairFromEnv("RELAYER_RECIPIENT").publicKey,
      relayerFee,
    );

    return relayer;
  }
  return relayer;
}
