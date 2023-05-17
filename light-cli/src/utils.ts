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
  TestRelayer,
  TRANSACTION_MERKLE_TREE_KEY,
  User,
} from "light-sdk";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

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
  provider: anchor.AnchorProvider
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
  try {

    const secretKey = bs58.decode(JSON.parse(getSecretKey()));

    let keypair: solana.Keypair = solana.Keypair.fromSecretKey(secretKey);

    return keypair;
  } catch (e: any) {
    throw new Error("secret key not found or corrupted!");
  }
};

export const setAnchorProvider = async (): Promise<anchor.AnchorProvider> => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = await getrpcUrl();

  const providerAnchor = anchor.AnchorProvider.local(
    await getrpcUrl(),
    confirmConfig
  );

  anchor.setProvider(providerAnchor);

  return providerAnchor;
};

export const getLightProvider = async (payer?: solana.Keypair) => {
  if (!provider) {
    const relayer = await getRelayer();

    await setAnchorProvider();

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

  console.log("loading the user ===========>");

  return await User.init({ provider });
};

export const getRelayer = async () => {
  if (!relayer) {
    const wallet = readWalletFromFile();
    relayer = new TestRelayer(
      wallet.publicKey,
      new solana.PublicKey(getLookUpTable() || ""),
      getRelayerRecipient(),
      RELAYER_FEES
    );

    return relayer;
  }
  return relayer;
};

type Config = {
  rpcUrl: string;
  relayerUrl: string;
  secretKey: string;
  relayerRecipient: string;
  lookUpTable: string;
  payer: string;
};

export const getrpcUrl = (): string => {
  const config = getConfig();
  return config.rpcUrl;
};

export const setrpcUrl = (url: string): void => {
  setConfig({ rpcUrl: url });
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
