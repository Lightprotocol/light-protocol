// nuke redis db entries

import { sleep } from "@lightprotocol/zk.js";
import { DB_VERSION } from "./src/config";
import { getTransactions } from "./src/db/redis";

(async () => {
  console.log("NUKING DB IN 10 SECONDS!");
  await sleep(1 * 1000);
  let { job } = await getTransactions(DB_VERSION);

  await job.updateData({ transactions: [] });
  let { job: job2 } = await getTransactions(DB_VERSION);
  console.log("job2", job2.data.transactions.length);
  process.exit(0);
})();
