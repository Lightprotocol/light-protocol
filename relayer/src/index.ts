import {
  Relayer,
  confirmConfig,
  createTestAccounts,
  initLookUpTableFromFile,
  setUpMerkleTree,
} from "light-sdk";

import express from "express";
import { setAnchorProvider } from "utils/provider";
import { port } from "config";

const app = express();

// Add CORS headers


var relayer;

(async () => {

  const providerAnchor = await setAnchorProvider()
  // TODO: use updated -- buildscript -> add relayer tests
  await createTestAccounts(providerAnchor.connection);

  await initLookUpTableFromFile(providerAnchor);
    
  await setUpMerkleTree(providerAnchor);
  /// *** this is not really necessary at this point *** TODO: remove
 
  await providerAnchor!.connection.confirmTransaction(
    await providerAnchor!.connection.requestAirdrop(
      relayer.accounts.relayerRecipient,
      1_000_000,
    ),
    "confirmed",
  );
  
})();

app.listen(port, async () => {
  console.log(`Webserver started on port ${port}`);
  console.log("rpc:", process.env.RPC_URL);
});
