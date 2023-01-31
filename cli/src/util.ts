import * as fs from "fs";
import * as solana from "@solana/web3.js";
import {
  AUTHORITY,
  confirmConfig,
  getLightInstance,
  Keypair,
  User,
} from "light-sdk";

export const createNewWallet = () => {
  const keypair: solana.Keypair = solana.Keypair.generate();
  const secretKey: solana.Ed25519SecretKey = keypair.secretKey;
  try {
    fs.writeFileSync(
      "./cache/secret.txt",
      JSON.stringify(Array.from(secretKey))
    );
    console.log("- secret created and cached");
    return keypair;
  } catch (e: any) {
    throw new Error("error writing secret.txt");
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
    let data: string = fs.readFileSync("./cache/secret.txt", "utf8");
    secretKey = JSON.parse(data);

    let asUint8Array: Uint8Array = new Uint8Array(secretKey);
    let keypair: solana.Keypair = solana.Keypair.fromSecretKey(asUint8Array);

    console.log("Wallet found!");
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
    payerSecret: Array.from(user.payer.secretKey),
    utxos: user.utxos,
  };

  fs.writeFileSync("./cache/user.txt", JSON.stringify(userToCache));
  console.log("- user cached");
};

// simulates state fetching.
export const readUserFromFile = async () => {
  let cachedUser: {
    seed: string;
    payerSecret: Array<number>;
    utxos: Array<any>;
  };

  try {
    let data: string = fs.readFileSync("./cache/user.txt", "utf8");
    cachedUser = JSON.parse(data);
  } catch (e: any) {
    console.log("user.txt snot found!");
  }
  /** This is not needed in UI. just adjust to JSON stringify. */
  let asUint8Array: Uint8Array = new Uint8Array(cachedUser.payerSecret);
  let rebuiltUser = {
    seed: cachedUser.seed,
    payer: solana.Keypair.fromSecretKey(asUint8Array),
    utxos: cachedUser.utxos,
  };
  try {
    let lightInstance = await getLightInstance();
    let user = new User({ payer: rebuiltUser.payer, lightInstance });
    console.log("loading user...");
    //@ts-ignore
    await user.load(rebuiltUser);
    console.log("User built from state!");
    return user;
  } catch (e) {
    console.log("err:", e);
  }
};
