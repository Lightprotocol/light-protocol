import express from "express";
import { testSetup } from "./setup";
import { DB_VERSION, port } from "./config";
import { addCorsHeaders } from "./middleware";
import bodyParser from "body-parser";
import {
  getIndexedTransactions,
  initMerkleTree,
  initLookupTable,
  updateMerkleTree,
  handleRelayRequest,
  runIndexer,
} from "./services";
import { getTransactions } from "./db/redis";
require("dotenv").config();

const app = express();

app.use(addCorsHeaders);
app.use(bodyParser.json());

app.post("/updatemerkletree", updateMerkleTree);

app.get("/merkletree", initMerkleTree);

app.get("/lookuptable", initLookupTable);

app.post("/relayTransaction", handleRelayRequest);

app.get("/indexedTransactions", getIndexedTransactions);

app.listen(port, async () => {
  if (process.env.TEST_ENVIRONMENT) {
    await testSetup();
    // TODO: temporary!
    let { job } = await getTransactions(DB_VERSION);
    await job.updateData({ transactions: [] });
  }

  runIndexer();

  console.log(`Webserver started on port ${port}`);
  console.log("rpc:", process.env.RPC_URL);
});
