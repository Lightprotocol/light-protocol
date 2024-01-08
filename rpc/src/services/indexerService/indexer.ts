import { Job } from "bullmq";
import { Connection } from "@solana/web3.js";
import { searchBackward, searchForward } from "./search";
import { MIN_INDEXER_SLOT } from "../../config";
import { RpcIndexedTransaction } from "@lightprotocol/zk.js";

function mergeAndSortTransactions(
  dbTransactions: RpcIndexedTransaction[],
  newTransactions: RpcIndexedTransaction[][],
) {
  const mergedTransactions: RpcIndexedTransaction[] = dbTransactions.concat(
    ...newTransactions,
  );
  const dedupedTransactions = mergedTransactions.reduce(
    (acc: RpcIndexedTransaction[], cur: RpcIndexedTransaction) => {
      if (
        cur &&
        !acc.find(
          (item) => item.transaction.signature === cur.transaction.signature,
        )
      ) {
        acc.push(cur);
      }
      return acc;
    },
    [],
  );
  dedupedTransactions.sort(
    (a: RpcIndexedTransaction, b: RpcIndexedTransaction) =>
      b.transaction.blockTime - a.transaction.blockTime,
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
    let olderTransactions: RpcIndexedTransaction[] = [];
    let oldestFetchedSignature: string | null = null;
    let continueBackwardFill = false;
    /// fillBackward is true when the indexer is started for the first time
    /// we continue to fill backward until searchBackward returns no more transactions,
    /// after which fillBackward never becomes true again.
    /// we fill backward until at least 1 tx is found.
    if (fillBackward || job.data.transactions.length === 0) {
      const result = await searchBackward(job, connection);
      olderTransactions = result.olderTransactions;
      oldestFetchedSignature = result.oldestFetchedSignature;

      if (olderTransactions.length > 0) continueBackwardFill = true;
    }

    const newerTransactions: RpcIndexedTransaction[] = await searchForward(
      job,
      connection,
    );

    const dedupedTransactions: RpcIndexedTransaction[] =
      mergeAndSortTransactions(job.data.transactions, [
        olderTransactions,
        newerTransactions,
      ]);
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
      oldestFetchedSignature: oldestFetchedSignature
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
  transactions: RpcIndexedTransaction[],
  minBlockTime: number,
) {
  return transactions.filter((trx) => trx.transaction.blockTime > minBlockTime);
}
