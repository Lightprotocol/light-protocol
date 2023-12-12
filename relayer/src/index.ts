import express from "express";
import { DB_VERSION, LOCAL_TEST_ENVIRONMENT, port } from "./config";
import { addCorsHeaders } from "./middleware";
import bodyParser from "body-parser";
import {
  getIndexedTransactions,
  buildMerkleTree,
  handleRelayRequest,
  runIndexer,
  getLookUpTable,
  getRelayerInfo,
} from "./services";
import { getTransactions } from "./db/redis";
import { createTestAccounts } from "@lightprotocol/zk.js";
import { getAnchorProvider } from "./utils/provider";

import { fundRelayer, lookUpTableIsInited } from "./setup";
import { AccountError, AccountErrorCode } from "./errors";
require("dotenv").config();

const app = express();

app.use(addCorsHeaders);
app.use(bodyParser.json());

app.get("/", async (_req: any, res: any) => {
  try {
    return res.status(200).json({ status: "gm." });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error", message: e.message });
  }
});

app.get("/getBuiltMerkletree", buildMerkleTree);

app.get("/lookuptable", getLookUpTable);

app.post("/relayTransaction", handleRelayRequest);

app.get("/indexedTransactions", getIndexedTransactions);

app.get("/getRelayerInfo", getRelayerInfo);

app.listen(port, async () => {
  console.log("Starting relayer...");
  const anchorProvider = await getAnchorProvider();

  /// We always expect the environment variable to be set to a valid and initialized lookuptable pubkey
  /// In local tests, we preload the hardcoded pubkey with `light test-validator`
  if (!(await lookUpTableIsInited(anchorProvider)))
    throw new AccountError(
      AccountErrorCode.LOOK_UP_TABLE_NOT_INITIALIZED,
      "startup",
    );

  /// Should only run in local tests.
  /// TODO: consider moving to a separate setup script for tests where relayer is involved
  if (LOCAL_TEST_ENVIRONMENT) {
    console.log("Funding relayer...");
    await fundRelayer();
    console.log("Setting up test environment...");
    await createTestAccounts(anchorProvider.connection);
    console.log("Test environment setup completed!");
    const { job } = await getTransactions(DB_VERSION);
    await job.updateData({ transactions: [] });
  }

  runIndexer();

  console.log(`Webserver started on port ${port}`);
  console.log("rpc:", process.env.RPC_URL);
});
