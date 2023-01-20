import * as fs from "fs";
import * as solana from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { User } from "light-sdk";
// import * as light from "light-sdk";
const circomlibjs = require("circomlibjs");

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

const decryptedUtxos: Array<Object> = [
  { test: "testString" },
  232323,
  "string",
];
export const saveUserToFile = ({
  signature,
  utxos,
}: {
  signature: Array<number>;
  utxos: Array<Object>;
}) => {
  fs.writeFileSync("./cache/signature.txt", JSON.stringify(signature));
  console.log("- signature cached");

  // TODO: encrypt user utxos
  fs.writeFileSync("./cache/utxos.txt", JSON.stringify(utxos));
  console.log("- utxos cached");
};

// simulates state fetching.
export const readUserFromFile = async () => {
  let signature: Array<number>;
  let decryptedUtxos: Array<Object> = [];
  try {
    let data: string = fs.readFileSync("./cache/signature.txt", "utf8");
    console.log(data);
    signature = JSON.parse(data);
  } catch (e: any) {
    console.log("signature.txt not found!");
  }
  try {
    let data: string = fs.readFileSync("./cache/utxos.txt", "utf8");
    console.log(JSON.parse(data));
    decryptedUtxos = JSON.parse(data);
  } catch (e: any) {
    console.log("utxos.txt not found!");
  }

  const signatureArray = Array.from(signature);
  // TODO: fetch and find user utxos (decr, encr)
  saveUserToFile({ signature: signatureArray, utxos: decryptedUtxos });

  const poseidon = await circomlibjs.buildPoseidonOpt();
  // TODO: add utxos to user..., add balance etc all to user account, also keys,
  // TODO: User: add "publickey functionality"
  //   const user = new User(
  //     poseidon,
  //     new anchor.BN(signatureArray).toString("hex")
  //     wallet
  //   );
  console.log("User logged in!");

  // create keys from signature

  // return user object
};
