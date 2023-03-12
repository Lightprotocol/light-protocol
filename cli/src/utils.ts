import * as fs from "fs";

import * as solana from "@solana/web3.js";
import {
  AUTHORITY,
  confirmConfig,
  getLightInstance,
  Account,
  User,
  Provider,
} from "light-sdk";
var getDirName = require("path").dirname;

export const createNewWallet = () => {
  const keypair: solana.Keypair = solana.Keypair.generate();
  const secretKey: solana.Ed25519SecretKey = keypair.secretKey;
  try {
    fs.mkdirSync(getDirName("./light-test-cache/secret.txt"));
    fs.writeFileSync(
      "./light-test-cache/secret.txt",
      JSON.stringify(Array.from(secretKey))
    );
    console.log("- secret created and cached");
    return keypair;
  } catch (e: any) {
    fs.crea;
    throw new Error(`error writing secret.txt: ${e}`);
  }
};

export async function getAirdrop(wallet: solana.Keypair) {
  const connection = getConnection();

  let balance = await connection.getBalance(wallet.publicKey, "confirmed");
  console.log(`balance ${balance} for ${wallet.publicKey.toString()}`);
  if (balance <= 50_000) {
    let amount = 10_000_000_000;
    let res = await connection.requestAirdrop(wallet.publicKey, amount);
    await connection.confirmTransaction(res, "confirmed");

    let Newbalance = await connection.getBalance(wallet.publicKey);

    console.assert(Newbalance == balance + amount, "airdrop failed");
    // subsidising transactions
    let txTransfer1 = new solana.Transaction().add(
      solana.SystemProgram.transfer({
        fromPubkey: wallet.publicKey,
        toPubkey: AUTHORITY,
        lamports: 1_000_000_000,
      })
    );
    await solana.sendAndConfirmTransaction(
      connection,
      txTransfer1,
      [wallet],
      confirmConfig
    );
  } else {
    console.log("no airdrop needed");
  }
}

export const getConnection = () =>
  new solana.Connection("http://127.0.0.1:8899");

export const readWalletFromFile = () => {
  let secretKey: Array<number> = [];
  try {
    let data: string = fs.readFileSync("./light-test-cache/secret.txt", "utf8");
    secretKey = JSON.parse(data);

    let asUint8Array: Uint8Array = new Uint8Array(secretKey);
    let keypair: solana.Keypair = solana.Keypair.fromSecretKey(asUint8Array);

    console.log("Wallet found!", keypair.publicKey.toString());
    return keypair;
  } catch (e: any) {
    throw new Error("secret.txt not found or corrupted!");
  }
};

export const saveUserToFile = ({ user }: { user: User }) => {
  /**
   * This represents the UIs state. (not localstorage!)
   * This should just store the whole user object.
   * TODO: store whole object (fix JSON serialization)
   * */
  let userToCache = {
    //@ts-ignore
    seed: user.seed,
    payerSecret: Array.from(user.provider.nodeWallet.secretKey),
    utxos: user.utxos,
  };

  fs.writeFileSync("./light-test-cache/user.txt", JSON.stringify(userToCache));
  console.log("- user cached");
};

// simulates state fetching.
export const readUserFromFile = async () => {
  // TODO: adapt to provider
  let cachedUser: {
    seed: string;
    payerSecret: Array<number>;
    utxos: Array<any>;
  };

  try {
    let data: string = fs.readFileSync("./light-test-cache/user.txt", "utf8");
    cachedUser = JSON.parse(data);
  } catch (e: any) {
    console.log("user.txt snot found!");
  }

  /** This is not needed in UI. just adjust to JSON stringify. */
  let asUint8Array: Uint8Array = new Uint8Array(cachedUser.payerSecret);
  try {
    const provider = await Provider.native(
      solana.Keypair.fromSecretKey(asUint8Array)
    );
    let state = {
      seed: cachedUser.seed,
      utxos: cachedUser.utxos,
    };

    console.log("loading user...");
    const user = await User.load(provider, state);
    //@ts-ignore
    console.log("✔️ User built from state!");
    return user;
  } catch (e) {
    console.log("err readUserFromFile:", e);
  }
};
