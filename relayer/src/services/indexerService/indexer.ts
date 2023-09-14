import { Job } from "bullmq";
import { Connection } from "@solana/web3.js";
import { IndexedTransaction } from "@lightprotocol/zk.js";
import { searchBackward, searchForward } from "./search";
import { MIN_INDEXER_SLOT } from "../../config";

function mergeAndSortTransactions(
  dbTransactions: IndexedTransaction[],
  newTransactions: IndexedTransaction[][],
) {
  let mergedTransactions: IndexedTransaction[] = dbTransactions.concat(
    ...newTransactions,
  );
  let dedupedTransactions = mergedTransactions.reduce(
    (acc: IndexedTransaction[], cur: IndexedTransaction) => {
      if (cur && !acc.find((item) => item.signature === cur.signature)) {
        acc.push(cur);
      }
      return acc;
    },
    [],
  );
  dedupedTransactions.sort(
    (a: IndexedTransaction, b: IndexedTransaction) => b.blockTime - a.blockTime,
  );
  return dedupedTransactions;
}

export async function indexTransactions({
  job,
  connection,
}: {
  job: Job;
  connection: Connection;
}) {
  try {
    const olderTransactions: IndexedTransaction[] = await searchBackward(
      job,
      connection,
    );

    const newerTransactions: IndexedTransaction[] = await searchForward(
      job,
      connection,
    );

    let dedupedTransactions: IndexedTransaction[] = mergeAndSortTransactions(
      job.data.transactions,
      [olderTransactions, newerTransactions],
    );
    console.log(
      `new total: ${dedupedTransactions.length} transactions old: ${job.data.transactions.length}, older: ${olderTransactions.length}, newer: ${newerTransactions.length}`,
    );

    let filteredByDeploymentVersion = filterTransactionsByMinBlockTime(
      dedupedTransactions,
      MIN_INDEXER_SLOT,
    );

    await job.updateData({
      transactions: filteredByDeploymentVersion,
      lastFetched: Date.now(),
    });
  } catch (e) {
    console.log("restarting indexer -- crash reason:", e);
  }
}

// This function is used to exclude transactions which have been executed before a certain block.
// We need this for testnet to exclude transactions of an old merkle tree.
function filterTransactionsByMinBlockTime(
  transactions: IndexedTransaction[],
  minBlockTime: number,
) {
  return transactions.filter((trx) => trx.blockTime > minBlockTime);
}
