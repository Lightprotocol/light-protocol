import {
  createTestAccounts,
  initLookUpTable,
  useWallet,
  airdropSol,
} from "@lightprotocol/zk.js";
import { getAnchorProvider, getKeyPairFromEnv } from "../utils/provider";
import { PublicKey } from "@solana/web3.js";
import { readFileSync, writeFileSync } from "fs";

export const testSetup = async () => {
  const providerAnchor = await getAnchorProvider();
  // TODO: use updated -- buildscript -> add relayer tests
  await createTestAccounts(providerAnchor.connection);

  let lookUpTable;
  const path = "lookUpTable.txt";
  try {
    let lookUpTableRead = new PublicKey(readFileSync(path, "utf8"));
    let lookUpTableInfoInit = await providerAnchor.connection.getAccountInfo(
      lookUpTableRead,
    );
    if (lookUpTableInfoInit) {
      lookUpTable = lookUpTableRead;
    }
  } catch (e) {
    console.log(".txt not found", e);
  }
  if (!lookUpTable) {
    console.log("initing lookuptable...");
    let wallet = useWallet(getKeyPairFromEnv("KEY_PAIR"));
    let balance = await wallet.connection.getBalance(wallet.publicKey);
    console.log("BALANCE PAYER", balance);
    await airdropSol({
      provider: providerAnchor,
      lamports: 1000 * 1e9,
      recipientPublicKey: wallet.publicKey,
    });
    balance = await wallet.connection.getBalance(wallet.publicKey);
    console.log("BALANCE PAYER", balance);
    lookUpTable = await initLookUpTable(wallet);

    writeFileSync(path, lookUpTable.toString(), "utf8");
  }
};
