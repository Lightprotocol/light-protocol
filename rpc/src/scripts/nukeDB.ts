// nuke redis db entries
import { sleep } from "@lightprotocol/zk.js";
import { DB_VERSION } from "../config";
import { getTransactions } from "../db/redis";

(async () => {
  console.log("NUKING DB IN 5 SECONDS!");
  await sleep(5 * 1000);
  const { job } = await getTransactions(DB_VERSION);

  await job.updateData({ transactions: [] });
  const { job: job2 } = await getTransactions(DB_VERSION);
  console.log("job2", job2.data.transactions.length);
  process.exit(0);
})();
