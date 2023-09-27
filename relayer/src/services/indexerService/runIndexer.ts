import { DB_VERSION, SECONDS } from "../../config";
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
    if (initialSync) await sleep(3 * SECONDS);
    else await sleep(5 * SECONDS);
    const { job } = await getTransactions(DB_VERSION);
    const url = process.env.RPC_URL;
    if (!url) throw new Error("Environment variable RPC_URL not set");
    const connection = new Connection(url, "confirmed");

    if (job) {
      console.log(
        `transactions indexed in db v${DB_VERSION}: ${job.data.transactions.length}`,
      );
    }
    await indexTransactions({
      job,
      connection,
    });
    if (rounds !== 0) {
      laps++;
    }
  }
}
