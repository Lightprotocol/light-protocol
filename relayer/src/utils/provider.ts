import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { relayerFee, relayerFeeRecipient, relayerPayer, rpcPort } from "config";
import {
  ADMIN_AUTH_KEYPAIR,
  confirmConfig,
  Provider,
  Relayer,
} from "light-sdk";

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
  const relayer = new Relayer(
    relayerPayer.publicKey,
    new PublicKey(""),
    relayerFeeRecipient.publicKey,
    relayerFee,
  );

  const provider = await Provider.init({ wallet: ADMIN_AUTH_KEYPAIR, relayer });

  await provider.provider!.connection.confirmTransaction(
    await provider.provider!.connection.requestAirdrop(
      relayer.accounts.relayerRecipient,
      1_000_000,
    ),
    "confirmed",
  );
  return provider;
};
