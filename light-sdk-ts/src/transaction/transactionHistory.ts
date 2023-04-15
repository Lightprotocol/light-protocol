import {
  ConfirmedSignaturesForAddress2Options,
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
import { DecodedData, indexedTransaction } from "../types";

/**
 * @async
 * @function processTransaction
 * @param {ParsedTransactionWithMeta} tx - The transaction object to process.
 * @param {indexedTransaction[]} transactions - An array to which the processed transaction data will be pushed.
 * @returns {Promise<void>}
 * This functions takes the transactionMeta and extracts relevant data from it,
 * including the signature, instruction parsed data, account keys, and transaction type.
 */
async function processTransaction(
  tx: ParsedTransactionWithMeta,
  transactions: indexedTransaction[],
) {
  // check if transaction contains the meta data or not , else return without processing transaction
  if (!tx || !tx.meta || tx.meta.err) return;

  const signature = tx.transaction.signatures[0];

  const solTokenPool = REGISTERED_POOL_PDA_SOL;

  const accountKeys = tx.transaction.message.accountKeys;

  // gets the index of REGISTERED_POOL_PDA_SOL in the accountKeys array
  const solTokenPoolIndex = accountKeys.findIndex(
    (item: ParsedMessageAccount) => {
      const itemStr =
        typeof item === "string" || item instanceof String
          ? item
          : item.pubkey.toBase58();
      return itemStr === solTokenPool.toBase58();
    },
  );

  let amount = new BN(
    tx.meta.postBalances[solTokenPoolIndex] -
      tx.meta.preBalances[solTokenPoolIndex],
  );

  // coder for decoding the incoming instructions of transaction
  const coder = new BorshCoder(IDL_VERIFIER_PROGRAM_ZERO);

  let from = PublicKey.default;
  let to = PublicKey.default;
  let relayerRecipientSol = PublicKey.default;
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

      // check if the decodedData from the transaction instruction is not null
      if (data) {
        const relayerFee = data.data["relayerFee"];
        const commitment = new BN(data.data["leaves"][0]).toString();
        const encryptedUtxos = data.data["encryptedUtxos"];
        const leaves = data.data["leaves"];
        const nullifiers = data.data["nullifiers"];

        amountSpl = new BN(data.data["publicAmountSpl"]);
        amountSol = new BN(data.data["publicAmountSol"]);
        amount = new BN(amount);

        // UNSHIEDL | TRANSFER
        if (amount.lt(new BN(0))) {
          amountSpl = amountSpl.sub(FIELD_SIZE).mod(FIELD_SIZE).abs();

          amountSol = amountSol
            .sub(FIELD_SIZE)
            .mod(FIELD_SIZE)
            .abs()
            .sub(relayerFee);

          amount = new BN(amount).abs().sub(relayerFee);

          type =
            amountSpl.toString() === "0" && amountSol.toString() === "0"
              ? // TRANSFER
                Action.TRANSFER
              : // UNSHIELD
                Action.UNSHIELD;

          if (type === Action.UNSHIELD) {
            to = accountKeys[1].pubkey;

            from = amountSpl.gt(new BN(0))
              ? // SPL
                accountKeys[10].pubkey
              : // SOL
                accountKeys[9].pubkey;
          }

          tx.meta.postBalances.forEach((el: any, index: any) => {
            if (
              new BN(tx.meta!.postBalances[index])
                .sub(new BN(tx.meta!.preBalances[index]))
                .eq(relayerFee)
            ) {
              relayerRecipientSol = accountKeys[index].pubkey;
            }
          });
        }
        // SHIELD
        else if (amount.gt(new BN(0)) || amountSpl.gt(new BN(0))) {
          from = accountKeys[0].pubkey;
          to = accountKeys[10].pubkey;
          type = Action.SHIELD;
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
          relayerRecipientSol,
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

/**
 * Fetches transactions for the specified merkleTreeProgramId in batches.
 * This function will handle retries and sleep to prevent rate-limiting issues.
 *
 * @param {Connection} connection - The Connection object to interact with the Solana network.
 * @param {PublicKey} merkleTreeProgramId - The PublicKey of the Merkle tree program.
 * @param {ConfirmedSignaturesForAddress2Options} batchOptions - Options for fetching transaction batches,
 * including starting transaction signature (after), ending transaction signature (before), and batch size (limit).
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
  batchOptions: ConfirmedSignaturesForAddress2Options;
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
 * @param {ConfirmedSignaturesForAddress2Options} batchOptions - Options for fetching transaction batches,
 * including starting transaction signature (after), ending transaction signature (before), and batch size (limit).
 * @param {boolean} dedupe=false - Whether to deduplicate transactions or not.
 * @returns {Promise<indexedTransaction[]>} Array of indexedTransactions
 */

export const getRecentTransactions = async ({
  connection,
  batchOptions = {
    limit: 1,
    before: undefined,
    until: undefined,
  },
  dedupe = false,
}: {
  connection: Connection;
  batchOptions: ConfirmedSignaturesForAddress2Options;
  dedupe?: boolean;
}): Promise<indexedTransaction[]> => {
  const batchSize = 1000;
  const rounds = Math.ceil(batchOptions.limit! / batchSize);
  const transactions: indexedTransaction[] = [];

  let batchBefore = batchOptions.before;

  for (let i = 0; i < rounds; i++) {
    const batchLimit =
      i === rounds - 1 ? batchOptions.limit! - i * batchSize : batchSize;
    const lastSignature = await getTransactionsBatch({
      connection,
      merkleTreeProgramId,
      batchOptions: {
        limit: batchLimit,
        before: batchBefore,
        until: batchOptions.until,
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
