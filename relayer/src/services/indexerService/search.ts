import { Connection } from "@solana/web3.js";
import { Job } from "bullmq";
import { FORWARD_SEARCH_BATCH_SIZE, TX_BATCH_SIZE } from "../../config";

import {
  IndexedTransaction,
  fetchRecentTransactions,
} from "@lightprotocol/zk.js";

export async function searchForward(job: Job, connection: Connection) {
  if (job.data.transactions.length === 0) return [];
  let mostRecentTransaction = job.data.transactions.reduce(
    (a: IndexedTransaction, b: IndexedTransaction) =>
      a.blockTime > b.blockTime ? a : b,
  );

  let newerTransactions = await fetchRecentTransactions({
    connection,
    batchOptions: {
      limit: FORWARD_SEARCH_BATCH_SIZE,
      until: mostRecentTransaction.signature,
    },
  });
  return newerTransactions;
}

export async function searchBackward(job: Job, connection: Connection) {
  if (job.data.transactions.length === 0) {
    let olderTransactions = await fetchRecentTransactions({
      connection,
      batchOptions: {
        limit: TX_BATCH_SIZE,
      },
    });
    return olderTransactions;
  } else {
    let oldestTransaction = job.data.transactions.reduce(
      (a: IndexedTransaction, b: IndexedTransaction) =>
        a.blockTime < b.blockTime ? a : b,
    );

    let olderTransactions: IndexedTransaction[] = await fetchRecentTransactions(
      {
        connection,
        batchOptions: {
          limit: TX_BATCH_SIZE,
          before: oldestTransaction.signature,
        },
      },
      undefined,
      job.data.transactions.ll,

    );
    return olderTransactions;
  }
}
