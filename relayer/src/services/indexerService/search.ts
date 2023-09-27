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

  let { transactions: newerTransactions } = await fetchRecentTransactions({
    connection,
    batchOptions: {
      limit: FORWARD_SEARCH_BATCH_SIZE,
      until: mostRecentTransaction.signature,
    },
  });
  return newerTransactions;
}

/*
 * Search backward from the most oldestFetchedSignature in the database.
 * This is not necessarily the oldestTransactions of the db. (if previous ones filterd out before)
 * If there are no transactions in the database, search backward from the most recent transaction in the chain.
 */

export async function searchBackward(
  job: Job,
  connection: Connection,
): Promise<{
  olderTransactions: IndexedTransaction[];
  oldestFetchedSignature: string;
}> {
  if (job.data.transactions.length === 0) {
    let { transactions: olderTransactions, oldestFetchedSignature } =
      await fetchRecentTransactions({
        connection,
        batchOptions: {
          limit: TX_BATCH_SIZE,
        },
      });
    return { olderTransactions, oldestFetchedSignature };
  } else {
    let previousOldestFetchedSignature = job.data.oldestFetchedSignature;

    let { transactions: olderTransactions, oldestFetchedSignature } =
      await fetchRecentTransactions({
        connection,
        batchOptions: {
          limit: TX_BATCH_SIZE,
          before: previousOldestFetchedSignature,
        },
      });
    return { olderTransactions, oldestFetchedSignature };
  }
}
