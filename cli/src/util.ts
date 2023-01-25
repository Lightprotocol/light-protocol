import * as fs from "fs";
import * as solana from "@solana/web3.js";
import { getLightInstance, User } from "light-sdk";

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
export const saveUserToFile = ({ user }: { user: User }) => {
  fs.writeFileSync("./cache/user.txt", JSON.stringify(user));
  console.log("- user cached");
};

// simulates state fetching.
export const readUserFromFile = async () => {
  let cachedUser: User;

  try {
    let data: string = fs.readFileSync("./cache/user.txt", "utf8");
    console.log(data);
    cachedUser = JSON.parse(data);
  } catch (e: any) {
    console.log("user.txt not found!");
  }

  let lightInstance = await getLightInstance();
  let user = new User({ lightInstance });
  await user.load(cachedUser);
  console.log("User built from cache!");
  return user;
};
