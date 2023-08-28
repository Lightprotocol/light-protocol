import {
  createTestAccounts,
  initLookUpTable,
  useWallet,
} from "@lightprotocol/zk.js";
import { getAnchorProvider, getKeyPairFromEnv } from "../utils/provider";
import { PublicKey } from "@solana/web3.js";
import { readFileSync, writeFileSync } from "fs";
import { RPC_URL } from "../config";

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
    console.log("initing lookuptable...");
    let wallet = useWallet(getKeyPairFromEnv("KEY_PAIR"), RPC_URL);

    lookUpTable = await initLookUpTable(wallet, anchorProvider);
    writeFileSync(path, lookUpTable.toString(), "utf8");
  }
}
