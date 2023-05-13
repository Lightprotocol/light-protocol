import * as fs from "fs";
import * as anchor from "@coral-xyz/anchor";
import * as solana from "@solana/web3.js";

import {
  ADMIN_AUTH_KEYPAIR,
  confirmConfig,
  MerkleTreeConfig,
  MESSAGE_MERKLE_TREE_KEY,
  Provider,
  Relayer,
  RELAYER_FEES,
  TRANSACTION_MERKLE_TREE_KEY,
  User,
} from "light-sdk";

require("dotenv").config();

var getDirName = require("path").dirname;
let provider: Provider;
let relayer: Relayer;

export const createNewWallet = () => {
  const keypair: solana.Keypair = solana.Keypair.generate();
  const secretKey: solana.Ed25519SecretKey = keypair.secretKey;
  try {
    setSecretKey(JSON.stringify(Array.from(secretKey)));
    console.log("- secret created and cached");
    return keypair;
  } catch (e: any) {
    throw new Error(`error writing secret.txt: ${e}`);
  }
};

export const getWalletConfig = async (
  provider: anchor.AnchorProvider,
): Promise<MerkleTreeConfig> => {

  let merkleTreeConfig = new MerkleTreeConfig({
    messageMerkleTreePubkey: MESSAGE_MERKLE_TREE_KEY,
    transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
    payer: getPayer(),
    connection: provider.connection,
  });

  await merkleTreeConfig.getMerkleTreeAuthorityPda();

  return merkleTreeConfig;
};

export const getConnection = () =>
  new solana.Connection("http://127.0.0.1:8899");

export const readWalletFromFile = () => {
  let secretKey: Array<number> = [];
  try {
    secretKey = JSON.parse(getSecretKey());

    let asUint8Array: Uint8Array = new Uint8Array(secretKey);
    let keypair: solana.Keypair = solana.Keypair.fromSecretKey(asUint8Array);

    console.log("Wallet found!", keypair.publicKey.toString());
    return keypair;
  } catch (e: any) {
    throw new Error("secret.txt not found or corrupted!");
  }
};

export const setAnchorProvider = async (): Promise<anchor.AnchorProvider> => {
  const configPath = "rpc-config.json";
  const rpcUrl = (process.env.ANCHOR_WALLET =
    process.env.HOME + "/.config/solana/id.json");
  process.env.ANCHOR_PROVIDER_URL = await getSolanaRpcUrl(); // runscript starts dedicated validator on this port.

  const providerAnchor = anchor.AnchorProvider.local(
    await getSolanaRpcUrl(),
    confirmConfig
  );

  anchor.setProvider(providerAnchor);

  return providerAnchor;
};

export const getLightProvider = async (payer?: solana.Keypair) => {
  if (!provider) {
    const relayer = await getRelayer();

    provider = await Provider.init({
      wallet: payer ? payer : readWalletFromFile(),
      relayer,
    });

    return provider;
  }
  return provider;
};

export const getUser = async () => {
  const provider = await getLightProvider();
  return User.init({ provider });
};

export const getRelayer = async () => {
  if (!relayer) {
    const wallet = readWalletFromFile();

    relayer = new Relayer(
      wallet.publicKey,
      new solana.PublicKey(process.env.LOOK_UP_TABLE || ""),
      getRelayerRecipient(),
      RELAYER_FEES
    );

    return relayer;
  }
  return relayer;
};

type Config = {
  solanaRpcUrl: string;
  relayerUrl: string;
  secretKey: string;
  relayerRecipient: string;
  lookUpTable: string;
  payer: string;
};

export const getSolanaRpcUrl = (): string => {
  const config = getConfig();
  return config.solanaRpcUrl;
};

export const setSolanaRpcUrl = (url: string): void => {
  setConfig({ solanaRpcUrl: url });
};

export const getRelayerUrl = (): string => {
  const config = getConfig();
  return config.relayerUrl;
};

export const setRelayerUrl = (url: string): void => {
  setConfig({ relayerUrl: url });
};

export const getSecretKey = () => {
  const config = getConfig();
  return config.secretKey;
};

export const setSecretKey = (key: string) => {
  setConfig({ secretKey: key });
};

export const getRelayerRecipient = () => {
  const config = getConfig();
  return new solana.PublicKey(config.relayerRecipient);
};

export const setRelayerRecipient = (address: string) => {
  setConfig({ relayerRecipient: address });
};

export const getLookUpTable = () => {
  const config = getConfig();
  return new solana.PublicKey(config.lookUpTable);
};

export const setLookUpTable = (address: string): void => {
  setConfig({ lookUpTable: address });
};

export const getPayer = () => {
  const config = getConfig();

  const payer = JSON.parse(config.payer);

  let asUint8Array: Uint8Array = new Uint8Array(payer);
  let keypair: solana.Keypair = solana.Keypair.fromSecretKey(asUint8Array);

  return keypair;
};

export const setPayer = (key: string) => {
  setConfig({ payer: key });
};

export const getConfig = (): Config => {
  try {
    const data = fs.readFileSync("config.json", "utf-8");
    return JSON.parse(data);
  } catch (err) {
    throw new Error("Failed to read configuration file");
  }
};

export const setConfig = (config: Partial<Config>): void => {
  try {
    const existingConfig = getConfig();
    const updatedConfig = { ...existingConfig, ...config };
    fs.writeFileSync("config.json", JSON.stringify(updatedConfig, null, 2));
  } catch (err) {
    throw new Error("Failed to update configuration file");
  }
};
