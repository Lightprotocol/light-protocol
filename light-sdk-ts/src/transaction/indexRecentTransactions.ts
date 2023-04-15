import {
  ConfirmedSignaturesForAddress2Options,
  Connection,
  ParsedMessageAccount,
  ParsedTransactionWithMeta,
  PublicKey,
  TokenBalance,
} from "@solana/web3.js";
import {
  merkleTreeProgramId,
  FIELD_SIZE,
  REGISTERED_POOL_PDA_SOL,
} from "../constants";

import { Action } from "./transaction";

import { getUpdatedSpentUtxos, sleep } from "../utils";
import { BN } from "@coral-xyz/anchor";
import {
  IndexedTransaction,
  UserIndexedTransaction,
  IndexedTransactionData,
} from "../types";
import { SPL_NOOP_ADDRESS } from "@solana/spl-account-compression";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Provider, TokenUtxoBalance } from "../wallet";
import { Utxo } from "../utxo";
import * as borsh from "@coral-xyz/borsh";

export class TransactionIndexerEvent {
  borshSchema = borsh.struct([
    borsh.vec(borsh.array(borsh.u8(), 32), "leaves"),
    borsh.array(borsh.u8(), 32, "publicAmountSpl"),
    borsh.array(borsh.u8(), 32, "publicAmountSol"),
    borsh.u64("relayerFee"),
    borsh.vec(borsh.u8(), "encryptedUtxos"),
    borsh.vec(borsh.array(borsh.u8(), 32), "nullifiers"),
    borsh.u64("firstLeafIndex"),
  ]);

  deserialize(buffer: Buffer): any | null {
    try {
      return this.borshSchema.decode(buffer);
    } catch (e) {
      return null;
    }
  }
}

/**
 * @async
 * @description This functions takes the IndexedTransaction and spentUtxos of user any return the filtered user indexed transactions
 * @function getUserIndexTransactions
 * @param {IndexedTransaction[]} indexedTransactions - An array to which the processed transaction data will be pushed.
 * @param {Provider} provider - provider class
 * @param {spentUtxos} Utxo[] - The array of user spentUtxos
 * @returns {Promise<void>}
 */
export const getUserIndexTransactions = async (
  indexedTransactions: IndexedTransaction[],
  provider: Provider,
  tokenBalances: Map<string, TokenUtxoBalance>,
) => {
  const transactionHistory: UserIndexedTransaction[] = [];

  const spentUtxos = getUpdatedSpentUtxos(tokenBalances);

  indexedTransactions.forEach((trx) => {
    const nullifierZero = new BN(trx.nullifiers[0]).toString();

    const nullifierOne = new BN(trx.nullifiers[1]).toString();

    const isFromUser =
      trx.from.toBase58() === provider.wallet.publicKey.toBase58();

    const inSpentUtxos: Utxo[] = [];
    const outSpentUtxos: Utxo[] = [];

    spentUtxos?.forEach((sUtxo) => {
      const matchesNullifier =
        sUtxo._nullifier === nullifierOne || sUtxo._nullifier === nullifierZero;

      let matchesCommitment = false;
      for (const leaf of trx.leaves) {
        if (!matchesCommitment) {
          matchesCommitment =
            sUtxo._commitment === new BN(leaf, "le").toString();
        }
      }

      if (matchesNullifier) {
        inSpentUtxos.push(sUtxo);
      }
      if (matchesCommitment) {
        outSpentUtxos.push(sUtxo);
      }
    });

    const found =
      isFromUser || inSpentUtxos.length > 0 || outSpentUtxos.length > 0;

    if (found) {
      transactionHistory.push({
        ...trx,
        inSpentUtxos,
        outSpentUtxos,
      });
    }
  });

  return transactionHistory.sort((a, b) => b.blockTime - a.blockTime);
};

/**
 * @async
 * @description This functions takes the indexer transaction event data and transaction,
 * including the signature, instruction parsed data, account keys, and transaction type.
 * @function processIndexedTransaction
 * @param {ParsedTransactionWithMeta} tx - The transaction object to process.
 * @param {IndexedTransaction[]} transactions - An array to which the processed transaction data will be pushed.
 * @returns {Promise<void>}
 */
async function processIndexedTransaction(
  event: IndexedTransactionData,
  transactions: IndexedTransaction[],
) {
  // check if transaction contains the meta data or not , else return without processing transaction
  const {
    tx,
    publicAmountSol,
    publicAmountSpl,
    relayerFee,
    firstLeafIndex,
    leaves,
    encryptedUtxos,
  } = event;

  if (!tx || !tx.meta || tx.meta.err) return;

  const signature = tx.transaction.signatures[0];

  const solTokenPool = REGISTERED_POOL_PDA_SOL;

  let accountKeys = tx.transaction.message.accountKeys;

  let type: Action = Action.SHIELD;
  let relayerRecipientSol = PublicKey.default;
  let from = PublicKey.default;
  let to = PublicKey.default;
  let verifier = accountKeys[2];

  // gets the index of REGISTERED_POOL_PDA_SOL in the accountKeys array
  let solTokenPoolIndex = accountKeys.findIndex(
    (item: ParsedMessageAccount) => {
      const itemStr =
        typeof item === "string" || item instanceof String
          ? item
          : item.pubkey.toBase58();
      return itemStr === solTokenPool.toBase58();
    },
  );

  let changeSolAmount = new BN(
    tx.meta.postBalances[solTokenPoolIndex] -
      tx.meta.preBalances[solTokenPoolIndex],
  );

  const encryptedUtxosBuffer = event.encryptedUtxos;

  let amountSpl = new BN(publicAmountSpl);
  let amountSol = new BN(publicAmountSol);

  const nullifiers = event.nullifiers;

  if (changeSolAmount.lt(new BN(0))) {
    amountSpl = amountSpl.sub(FIELD_SIZE).mod(FIELD_SIZE).abs();

    amountSol = amountSol.sub(FIELD_SIZE).mod(FIELD_SIZE).abs().sub(relayerFee);

    // amount = new BN(amount).abs().sub(relayerFee);
    changeSolAmount = new BN(changeSolAmount).abs().sub(relayerFee);

    type =
      amountSpl.eq(new BN(0)) && amountSol.eq(new BN(0))
        ? // TRANSFER
          Action.TRANSFER
        : // UNSHIELD
          Action.UNSHIELD;

    tx.meta.postBalances.forEach((el: any, index: number) => {
      if (
        new BN(tx.meta!.postBalances[index])
          .sub(new BN(tx.meta!.preBalances[index]))
          .eq(relayerFee)
      ) {
        relayerRecipientSol = accountKeys[index].pubkey;
      }
    });

    if (type === Action.UNSHIELD) {
      to = accountKeys[1].pubkey;

      from = amountSpl.gt(new BN(0))
        ? // SPL
          accountKeys[10].pubkey
        : // SOL
          accountKeys[9].pubkey;
    }
  } else if (changeSolAmount.gt(new BN(0)) || amountSpl.gt(new BN(0))) {
    type = Action.SHIELD;
    from = accountKeys[0].pubkey;
    to = accountKeys[10].pubkey;
  }

  transactions.push({
    blockTime: tx.blockTime! * 1000,
    signer: accountKeys[0].pubkey,
    signature,
    accounts: accountKeys,
    to,
    from: from,
    verifier,
    relayerRecipientSol,
    type,
    changeSolAmount,
    publicAmountSol: amountSol,
    publicAmountSpl: amountSpl,
    encryptedUtxos,
    leaves,
    nullifiers,
    relayerFee,
    firstLeafIndex,
  });

  return;
}

/**
 * @async
 * @description This functions takes the transactionMeta of  indexer events transactions and extracts relevant data from it
 * @function processIndexerEventsTransactions
 * @param {(ParsedTransactionWithMeta | null)[]} indexerEventsTransactions - An array of indexer event transactions to process
 * @returns {Promise<void>}
 */
const processIndexerEventsTransactions = async (
  indexerEventsTransactions: (ParsedTransactionWithMeta | null)[],
) => {
  const indexerTransactionEvents: IndexedTransactionData[] = [];

  indexerEventsTransactions.forEach((tx) => {
    if (
      !tx ||
      !tx.meta ||
      tx.meta.err ||
      !tx.meta.innerInstructions ||
      tx.meta.innerInstructions.length <= 0
    ) {
      return;
    }

    tx.meta.innerInstructions.forEach((ix) => {
      ix.instructions.forEach((ixInner: any) => {
        if (!ixInner.data) return;

        const data = bs58.decode(ixInner.data);

        const decodeData = new TransactionIndexerEvent().deserialize(data);

        if (decodeData) {
          indexerTransactionEvents.push({
            ...decodeData,
            tx,
          });
        }
      });
    });
  });

  return indexerTransactionEvents;
};

/**
 * @description Fetches transactions for the specified merkleTreeProgramId in batches
 * and process the incoming transaction using the processIndexedTransaction.
 * This function will handle retries and sleep to prevent rate-limiting issues.
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

  const indexerEventTransactions = txs.filter((tx: any) => {
    const accountKeys = tx.transaction.message.accountKeys;
    const splNoopIndex = accountKeys.findIndex((item: ParsedMessageAccount) => {
      const itemStr =
        typeof item === "string" || item instanceof String
          ? item
          : item.pubkey.toBase58();
      return itemStr === new PublicKey(SPL_NOOP_ADDRESS).toBase58();
    });

    if (splNoopIndex) {
      return txs;
    }
  });

  const indexerTransactionEvents = await processIndexerEventsTransactions(
    indexerEventTransactions,
  );

  indexerTransactionEvents.forEach((event) => {
    processIndexedTransaction(event!, transactions);
  });

  return lastSignature;
};

/**
 * @description Fetches recent transactions for the specified merkleTreeProgramId.
 * This function will call getTransactionsBatch multiple times to fetch transactions in batches.
 * @param {Connection} connection - The Connection object to interact with the Solana network.
 * @param {ConfirmedSignaturesForAddress2Options} batchOptions - Options for fetching transaction batches,
 * including starting transaction signature (after), ending transaction signature (before), and batch size (limit).
 * @param {boolean} dedupe=false - Whether to deduplicate transactions or not.
 * @returns {Promise<indexedTransaction[]>} Array of indexedTransactions
 */

export const indexRecentTransactions = async ({
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
}): Promise<IndexedTransaction[]> => {
  const batchSize = 1000;
  const rounds = Math.ceil(batchOptions.limit! / batchSize);
  const transactions: IndexedTransaction[] = [];

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

  return transactions.sort(
    (a, b) => a.firstLeafIndex.toNumber() - b.firstLeafIndex.toNumber(),
  );
};
