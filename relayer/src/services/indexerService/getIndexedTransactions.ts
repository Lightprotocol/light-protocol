import { IndexedTransaction } from "@lightprotocol/zk.js";
import { DB_VERSION } from "../../config";
import { indexQueue } from "../../db/redis";

export async function getIndexedTransactions(_req: any, res: any) {
  console.log("@getIndexedTransactions!");
  try {
    const version = DB_VERSION;
    const job = (await indexQueue.getWaiting())[version];
    return res
      .status(200)
      .json({ data: job.data.transactions, lastFetched: job.data.lastFetched });
  } catch (error) {
    console.log("getIndexedTransactions error:", error.message);
    return res.status(500).json({ status: "error", message: error.message });
  }
}
