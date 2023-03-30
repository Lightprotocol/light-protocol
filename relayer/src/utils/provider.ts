import * as anchor from "@coral-xyz/anchor";
import { rpcPort } from "config";
import { confirmConfig } from "light-sdk";

export const setAnchorProvider = async (): Promise<anchor.AnchorProvider> => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = `http://127.0.0.1:${rpcPort}`; // runscript starts dedicated validator on this port.

  const providerAnchor = anchor.AnchorProvider.local(
    `http://127.0.0.1:${rpcPort}`,
    confirmConfig,
  );

  anchor.setProvider(providerAnchor);
  return providerAnchor
};
