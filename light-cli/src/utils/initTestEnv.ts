import {
  ADMIN_AUTH_KEY,
  createTestAccounts,
  initLookUpTableFromFile,
  sleep,
} from "@lightprotocol/zk.js";
import {
  setAnchorProvider,
  setLookUpTable,
  setRelayerRecipient,
} from "./utils";
import { Keypair } from "@solana/web3.js";
import { exec, execSync } from "child_process";

export async function initTestEnv() {
  console.log("Performing setup tasks...\n");
  execSync("sh runScript.sh", { stdio: "inherit" });

  const anchorProvider = await setAnchorProvider();

  await createTestAccounts(anchorProvider.connection);

  const lookupTable = await initLookUpTableFromFile(anchorProvider);

  setLookUpTable(lookupTable.toString());

  const relayerRecipientSol = Keypair.generate().publicKey;

  setRelayerRecipient(relayerRecipientSol.toString());

  await anchorProvider.connection.requestAirdrop(
    relayerRecipientSol,
    2_000_000_000
  );
}

export async function initTestEnvIfNeeded() {
  try {
    const anchorProvider = await setAnchorProvider();
    // this request will fail if there is no local test validator running
    await anchorProvider.connection.getBalance(ADMIN_AUTH_KEY);
  } catch (error) {
    // launch local test validator and initialize test environment
    await initTestEnv();
  }
}
