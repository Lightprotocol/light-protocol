import { DB_VERSION, RPC_URL, SECONDS } from "../../config";
import { indexTransactions } from "./indexer";
import { Connection } from "@solana/web3.js";
import { getTransactions } from "../../db/redis";
import { sleep } from "@lightprotocol/zk.js";

export async function runIndexer(rounds: number = 0) {
  // initialize
  console.log("runIndexer initializing...");
  await getTransactions(DB_VERSION);
  console.log("initialized");
  var initialSync = false;
  var laps = -1;
  while (laps < rounds) {
    if (initialSync) await sleep(2 * SECONDS);
    else await sleep(5 * SECONDS);
    const { job } = await getTransactions(DB_VERSION);
    const RPC_connection = new Connection("http://127.0.0.1:8899", "confirmed");
    if (job) {
      console.log(
        `transactions indexed in db v${DB_VERSION}: ${job.data.transactions.length}`,
      );
    }
    await indexTransactions({
      job,
      RPC_connection,
      initialSync,
    });
    if (rounds !== 0) {
      // default = infinite = 0
      laps++;
    }
  }
}
