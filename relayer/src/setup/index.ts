import {
  createTestAccounts,
  initLookUpTable,
  useWallet,
  airdropSol,
} from "@lightprotocol/zk.js";
import {
  getAnchorProvider,
  getKeyPairFromEnv,
  getRelayer,
} from "../utils/provider";
import { PublicKey } from "@solana/web3.js";
import { readFileSync, writeFileSync } from "fs";
import { AnchorProvider, BN } from "@coral-xyz/anchor";
import { AIRDROP_DECIMALS, RPC_URL } from "../config";

export async function relayerSetup() {
  const anchorProvider = await getAnchorProvider();

  await createTestAccounts(anchorProvider.connection);

  let lookUpTable;
  const path = "lookUpTable.txt";
  try {
    let lookUpTableRead = new PublicKey(readFileSync(path, "utf8"));
    let lookUpTableInfoInit = await anchorProvider.connection.getAccountInfo(
      lookUpTableRead,
    );
    if (lookUpTableInfoInit) {
      lookUpTable = lookUpTableRead;
    }
  } catch (e) {
    console.log(".txt not found", e);
  }
  if (!lookUpTable) {
    console.log("initing lookuptable... rpc url relayer", RPC_URL);
    let wallet = useWallet(getKeyPairFromEnv("KEY_PAIR"), RPC_URL);
    // for (let sol = 0; sol < 2; sol++)
    await airdropSol({
      connection: anchorProvider.connection,
      lamports: 10 * AIRDROP_DECIMALS,
      recipientPublicKey: wallet.publicKey,
    });
    console.log(
      "PAYER RELAYER (initLookupTable): ",
      wallet.publicKey.toBase58(),
    );
    console.log(
      "wallet balance:",

      await anchorProvider.connection.getBalance(wallet.publicKey),
      "providerpayer:",
      anchorProvider.wallet.publicKey.toBase58(),
      await anchorProvider.connection.getBalance(
        anchorProvider.wallet.publicKey,
      ),
    );

    lookUpTable = await initLookUpTable(wallet, anchorProvider);
    writeFileSync(path, lookUpTable.toString(), "utf8");
  }
  try {
    await fundRelayer(anchorProvider);
  } catch (e) {
    console.log("fundRelayer e:", e);
  }
}

async function fundRelayer(anchorProvider: AnchorProvider) {
  console.log("con n", anchorProvider.connection);
  await airdropSol({
    connection: anchorProvider.connection,
    lamports: 10 * AIRDROP_DECIMALS,
    recipientPublicKey: getKeyPairFromEnv("KEY_PAIR").publicKey,
  });
  const relayer = await getRelayer();
  relayer.relayerFee = new BN(100_000);
  await airdropSol({
    connection: anchorProvider.connection,
    lamports: 10 * AIRDROP_DECIMALS,
    recipientPublicKey: relayer.accounts.relayerRecipientSol,
  });
}
