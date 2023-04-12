import {
  Connection,
  ParsedMessageAccount,
  ParsedTransactionWithMeta,
  PublicKey,
} from "@solana/web3.js";
import {
  merkleTreeProgramId,
  FIELD_SIZE,
  REGISTERED_POOL_PDA_SOL,
} from "../constants";

import { Action } from "./transaction";

import { sleep } from "../utils";
import { IDL_VERIFIER_PROGRAM_ZERO } from "../idls";
import { BorshCoder, BN } from "@coral-xyz/anchor";
import { DecodedData, historyTransaction } from "../types";

// TODO: to and from in the all transaction
// TODO: test-cases for transaction

/**
 * @async
 * @function processTransaction
 * @param {ParsedTransactionWithMeta} tx - The transaction object to process.
 * @param {historyTransaction[]} transactions - An array to which the processed transaction data will be pushed.
 * @returns {Promise<void>}
 * This functions takes the transactionMeta and extracts relevant data from it,
 * including the signature, instruction parsed data, account keys, and transaction type.
 */
async function processTransaction(
  tx: ParsedTransactionWithMeta,
  transactions: historyTransaction[],
) {
  if (!tx || !tx.meta || tx.meta.err) return;

  const signature = tx.transaction.signatures[0];
  const tokenPool = new PublicKey(REGISTERED_POOL_PDA_SOL);
  const accountKeys = tx.transaction.message.accountKeys;
  const i = accountKeys.findIndex((item: ParsedMessageAccount) => {
    const itemStr =
      typeof item === "string" || item instanceof String
        ? item
        : item.pubkey.toBase58();
    return itemStr === tokenPool.toBase58();
  });

  let amount: number | BN = tx.meta.postBalances[i] - tx.meta.preBalances[i];

  const coder = new BorshCoder(IDL_VERIFIER_PROGRAM_ZERO);

  let from: PublicKey;
  let to: PublicKey = accountKeys[2].pubkey;
  let type: Action;
  let amountSpl;
  let amountSol;

  if (!tx.meta.err) {
    const instructions = tx.transaction.message.instructions;
    for (const instruction of instructions) {
      // @ts-ignore
      const rawData = instruction.data;
      const data = coder.instruction.decode(
        rawData,
        "base58",
      ) as DecodedData | null;

      if (data) {
        const relayerFee = data.data["relayerFee"];
        const commitment = new BN(data.data["leaves"][0]).toString();
        const encryptedUtxos = data.data["encryptedUtxos"];
        const leaves = data.data["leaves"];
        const nullifiers = data.data["nullifiers"];

        if (amount < 0) {
          amountSpl = new BN(data.data["publicAmountSpl"])
            .sub(FIELD_SIZE)
            .mod(FIELD_SIZE)
            .abs();

          amountSol = new BN(data.data["publicAmountSol"])
            .sub(FIELD_SIZE)
            .mod(FIELD_SIZE)
            .abs();

          from = new PublicKey(REGISTERED_POOL_PDA_SOL);

          amount = new BN(amount).abs().sub(relayerFee);

          type =
            tx.transaction.message.accountKeys.length <= 16 &&
            i === 10 &&
            amountSpl.toString() === "0" &&
            amount.toString() === "0"
              ? Action.TRANSFER
              : Action.UNSHIELD;

          const toIndex = tx.meta.postBalances.findIndex(
            (el: any, index: any) => {
              return (
                tx.meta!.postBalances[index] - tx.meta!.preBalances[index] ===
                parseInt(amount.toString())
              );
            },
          );

          if (toIndex > 0) {
            to = accountKeys[toIndex].pubkey;
          }
        } else if (amount > 0 && (i === 10 || i === 11)) {
          amountSpl = new BN(data.data["publicAmountSpl"].slice(24, 32));
          amountSol = new BN(data.data["publicAmountSol"].slice(24, 32));
          from = accountKeys[0].pubkey;
          to = new PublicKey(REGISTERED_POOL_PDA_SOL);
          type = Action.SHIELD;

          amount = new BN(amount);
        } else {
          continue;
        }
        transactions.push({
          blockTime: tx.blockTime! * 1000,
          signer: accountKeys[0].pubkey,
          signature,
          accounts: accountKeys,
          to,
          from: from,
          type,
          amount,
          amountSol,
          amountSpl,
          commitment,
          encryptedUtxos,
          leaves,
          nullifiers,
          relayerFee,
        });
        break;
      }
    }
  }
}

type BatchOptions = {
  limit: number;
  before: any;
  until: any;
};

/**
 * Fetches transactions for the specified merkleTreeProgramId in batches.
 * This function will handle retries and sleep to prevent rate-limiting issues.
 *
 * @param {Connection} connection - The Connection object to interact with the Solana network.
 * @param {PublicKey} merkleTreeProgramId - The PublicKey of the Merkle tree program.
 * @param {BatchOptions} batchOptions - The options to use when fetching transaction batches.
 * @param {any[]} transactions - The array where the fetched transactions will be stored.
 * @returns {Promise<string>} - The signature of the last fetched transaction.
 */
const getTransactionsBatch = async ({
  connection,
  merkleTreeProgramId,
  batchOptions,
  transactions,
}: {
  connection: Connection;
  merkleTreeProgramId: PublicKey;
  batchOptions: BatchOptions;
  transactions: any;
}) => {
  const signatures = await connection.getConfirmedSignaturesForAddress2(
    new PublicKey(merkleTreeProgramId),
    batchOptions,
    "confirmed",
  );

  const lastSignature = signatures[signatures.length - 1];
  let txs: (ParsedTransactionWithMeta | null)[] = [];
  let index = 0;

  while (index < signatures.length) {
    try {
      const txsBatch = await connection.getParsedTransactions(
        signatures.slice(index, index + 25).map((sig) => sig.signature),
        {
          maxSupportedTransactionVersion: 0,
          commitment: "confirmed",
        },
      );

      if (!txsBatch.some((t) => !t)) {
        txs = txs.concat(txsBatch);
        index += 25;
      }
    } catch (e) {
      console.log("retry");
      await sleep(2000);
    }
  }

  txs.forEach((tx) => {
    processTransaction(tx!, transactions);
  });

  return lastSignature;
};

/**
 * Fetches recent transactions for the specified merkleTreeProgramId.
 * This function will call getTransactionsBatch multiple times to fetch transactions in batches.
 *
 * @param {Connection} connection - The Connection object to interact with the Solana network.
 * @param {number} limit - The maximum number of transactions to fetch.
 * @param {boolean} dedupe=false - Whether to deduplicate transactions or not.
 * @param {any} after=null - Fetch transactions after this value (optional).
 * @param {any} before=null - fetch transactions before this value (optional )
 * @returns {Promise<historyTransaction[]>} Array of historyTransactions
 */

export const getRecentTransactions = async ({
  connection,
  limit = 1,
  dedupe = false,
  after = null,
  before = null,
}: {
  connection: Connection;
  limit: number;
  dedupe?: boolean;
  after?: any;
  before?: any;
}): Promise<historyTransaction[]> => {
  const batchSize = 1000;
  const rounds = Math.ceil(limit / batchSize);
  const transactions: historyTransaction[] = [];

  let batchBefore = before;

  for (let i = 0; i < rounds; i++) {
    const batchLimit = i === rounds - 1 ? limit - i * batchSize : batchSize;
    const lastSignature = await getTransactionsBatch({
      connection,
      merkleTreeProgramId,
      batchOptions: {
        limit: batchLimit,
        before: batchBefore,
        until: after,
      },
      transactions,
    });

    if (!lastSignature) {
      break;
    }

    batchBefore = lastSignature.signature;
    await sleep(500);
  }
  return transactions.sort((a, b) => b.blockTime - a.blockTime);
};
