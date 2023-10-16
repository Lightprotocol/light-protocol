import { Job } from "bullmq";
import { Connection } from "@solana/web3.js";
import { IndexedTransaction } from "@lightprotocol/zk.js";
import { searchBackward, searchForward } from "./search";
import { MIN_INDEXER_SLOT } from "../../config";

function mergeAndSortTransactions(
  dbTransactions: IndexedTransaction[],
  newTransactions: IndexedTransaction[][],
) {
  const mergedTransactions: IndexedTransaction[] = dbTransactions.concat(
    ...newTransactions,
  );
  const dedupedTransactions = mergedTransactions.reduce(
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
  fillBackward,
}: {
  job: Job;
  connection: Connection;
  fillBackward: boolean;
}): Promise<{ continueBackwardFill: boolean }> {
  try {
    let olderTransactions: IndexedTransaction[] = [];
    let oldestFetchedSignature: string | null = null;
    let continueBackwardFill = false;
    /// fillBackward is true when the indexer is started for the first time
    /// we continue to fill backward until searchBackward returns no more transactions,
    /// after which fillBackward never becomes true again.
    if (fillBackward) {
      const result = await searchBackward(job, connection);
      olderTransactions = result.olderTransactions;
      oldestFetchedSignature = result.oldestFetchedSignature;

      if (olderTransactions.length > 0) continueBackwardFill = true;
    }

    const newerTransactions: IndexedTransaction[] = await searchForward(
      job,
      connection,
    );

    const dedupedTransactions: IndexedTransaction[] = mergeAndSortTransactions(
      job.data.transactions,
      [olderTransactions, newerTransactions],
    );
    console.log(
      `new total: ${dedupedTransactions.length} transactions old: ${job.data.transactions.length}, older: ${olderTransactions.length}, newer: ${newerTransactions.length}`,
    );

    const filteredByDeploymentVersion = filterTransactionsByMinBlockTime(
      dedupedTransactions,
      MIN_INDEXER_SLOT,
    );
    await job.updateData({
      transactions: filteredByDeploymentVersion,
      lastFetched: Date.now(),
      oldestFetchedSignature: fillBackward
        ? oldestFetchedSignature
        : job.data.oldestFetchedSignature,
    });
    return { continueBackwardFill };
  } catch (e) {
    console.log("restarting indexer -- crash reason:", e);
    return { continueBackwardFill: true };
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
