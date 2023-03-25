import {
  Relayer,
  confirmConfig,
  createTestAccounts,
  initLookUpTableFromFile,
  setUpMerkleTree,
} from "light-sdk";
import * as anchor from "@coral-xyz/anchor";
import express from "express";
import { corsMiddleware } from "middleware";
import router from "routes";
import { relayerFee, relayerFeeRecipient, relayerPayer, rpcPort } from "config";
const app = express();
const port = 3331;

// Add CORS headers
app.use(corsMiddleware);
app.use(router)

var relayer;

(async () => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = `http://127.0.0.1:${rpcPort}`; // runscript starts dedicated validator on this port.

  const providerAnchor = anchor.AnchorProvider.local(
    `http://127.0.0.1:${rpcPort}`,
    confirmConfig,
  );
  anchor.setProvider(providerAnchor);

  console.log("anchor provider set");

  // TODO: use updated -- buildscript -> add relayer tests
  await createTestAccounts(providerAnchor.connection);
  console.log("test accounts created");
  let LOOK_UP_TABLE = await initLookUpTableFromFile(providerAnchor);
  console.log("lookup table initialized");
  await setUpMerkleTree(providerAnchor);
  /// *** this is not really necessary at this point *** TODO: remove
  console.log("merkletree set up done");
  relayer = new Relayer(
    relayerPayer.publicKey,
    LOOK_UP_TABLE,
    relayerFeeRecipient.publicKey,
    relayerFee,
  );

  await providerAnchor!.connection.confirmTransaction(
    await providerAnchor!.connection.requestAirdrop(
      relayer.accounts.relayerRecipient,
      1_000_000,
    ),
    "confirmed",
  );
  console.log(
    "Relayer initialized",
    relayer.accounts.relayerPubkey.toBase58(),
    "relayerRecipient: ",
    relayer.accounts.relayerRecipient.toBase58(),
  );
})();

app.listen(port, async () => {
  console.log(`Webserver started on port ${port}`);
  console.log("rpc:", process.env.RPC_URL);
});
