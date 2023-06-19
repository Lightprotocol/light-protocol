import {
  Connection,
  ConfirmedSignaturesForAddress2Options,
  ParsedMessageAccount,
  ParsedTransactionWithMeta,
  PublicKey,
} from "@solana/web3.js";
import * as borsh from "@coral-xyz/borsh";
import { BN } from "@coral-xyz/anchor";
import { SPL_NOOP_ADDRESS } from "@solana/spl-account-compression";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  merkleTreeProgramId,
  FIELD_SIZE,
  REGISTERED_POOL_PDA_SOL,
} from "../constants";
import { sleep, getUpdatedSpentUtxos } from "../utils";
import { Provider, TokenUtxoBalance } from "../wallet";
import { Utxo } from "../utxo";
import { Action } from "./transaction";
import {
  IndexedTransaction,
  UserIndexedTransaction,
  IndexedTransactionData,
} from "../types";

// @matteo: borshSchema and the deserialize should be declared "static"
export class TransactionIndexerEvent {
  static borshSchema = borsh.struct([
    borsh.vec(borsh.array(borsh.u8(), 32), "leaves"),
    borsh.array(borsh.u8(), 32, "publicAmountSpl"),
    borsh.array(borsh.u8(), 32, "publicAmountSol"),
    borsh.u64("relayerFee"),
    borsh.vec(borsh.u8(), "encryptedUtxos"),
    borsh.vec(borsh.array(borsh.u8(), 32), "nullifiers"),
    borsh.u64("firstLeafIndex"),
    borsh.vecU8("message"),
  ]);

  static deserialize(buffer: Buffer): any | null {
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
// xonoxitron@matteo: filter/map run as twice as fast than forEach
export const getUserIndexTransactions = async (
  indexedTransactions: IndexedTransaction[],
  provider: Provider,
  tokenBalances: Map<string, TokenUtxoBalance>,
) => {
  const spentUtxos = getUpdatedSpentUtxos(tokenBalances);
  const transactionHistory: UserIndexedTransaction[] = [];

  for (const trx of indexedTransactions) {
    const nullifierZero = new BN(trx.nullifiers[0]).toString();
    const nullifierOne = new BN(trx.nullifiers[1]).toString();
    const isFromUser =
      trx.from.toBase58() === provider.wallet.publicKey.toBase58();

    const inSpentUtxos: Utxo[] = filterSpentUtxosByNullifiers(
      spentUtxos,
      nullifierZero,
      nullifierOne,
    );
    const leaves = trx.leaves.map((leaf) => leaf.toString()); // Convert leaves to strings
    const outSpentUtxos = filterSpentUtxosByLeaves(spentUtxos, leaves);

    const found =
      isFromUser || inSpentUtxos.length > 0 || outSpentUtxos.length > 0;

    if (found) {
      transactionHistory.push({
        ...trx,
        inSpentUtxos,
        outSpentUtxos,
      });
    }
  }

  return sortTransactionHistory(transactionHistory);
};

const filterSpentUtxosByNullifiers = (
  spentUtxos: Utxo[] | undefined,
  nullifierZero: string,
  nullifierOne: string,
) => {
  return (
    spentUtxos?.filter(
      (sUtxo) =>
        sUtxo._nullifier === nullifierOne || sUtxo._nullifier === nullifierZero,
    ) ?? []
  );
};

const filterSpentUtxosByLeaves = (
  spentUtxos: Utxo[] | undefined,
  leaves: string[],
) => {
  return (
    spentUtxos?.filter((sUtxo) => {
      for (const leaf of leaves) {
        if (sUtxo._commitment === new BN(leaf, "le").toString()) {
          return true;
        }
      }
      return false;
    }) ?? []
  );
};

const sortTransactionHistory = (
  transactionHistory: UserIndexedTransaction[],
) => {
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
  const {
    tx,
    publicAmountSol,
    publicAmountSpl,
    relayerFee,
    firstLeafIndex,
    leaves,
    encryptedUtxos,
    message,
  } = event;

  if (!tx || !tx.meta || tx.meta.err) {
    return;
  }

  const signature = tx.transaction.signatures[0];
  const solTokenPool = REGISTERED_POOL_PDA_SOL;
  const accountKeys = tx.transaction.message.accountKeys;
  const bn0 = new BN(0);
  const nullifiers = event.nullifiers;

  let type: Action = Action.SHIELD;
  let relayerRecipientSol: PublicKey = PublicKey.default;
  let from: PublicKey = PublicKey.default;
  let to: PublicKey = PublicKey.default;
  let verifier: PublicKey = PublicKey.default;
  let solTokenPoolIndex = -1;
  let changeSolAmount = new BN(0);
  let amountSpl = new BN(publicAmountSpl);
  let amountSol = new BN(publicAmountSol);

  if (tx.meta && tx.meta.preBalances && tx.meta.postBalances) {
    solTokenPoolIndex = accountKeys.findIndex(
      (item: ParsedMessageAccount | string) =>
        typeof item === "string"
          ? item === solTokenPool.toBase58()
          : item.pubkey.toBase58() === solTokenPool.toBase58(),
    );

    changeSolAmount = new BN(
      tx.meta.postBalances[solTokenPoolIndex] -
        tx.meta.preBalances[solTokenPoolIndex],
    );
  }

  if (changeSolAmount.lt(bn0)) {
    amountSpl = amountSpl.sub(FIELD_SIZE).mod(FIELD_SIZE).abs();
    amountSol = amountSol.sub(FIELD_SIZE).mod(FIELD_SIZE).abs().sub(relayerFee);
    changeSolAmount = changeSolAmount.abs().sub(relayerFee);

    type =
      amountSpl.eq(bn0) && amountSol.eq(bn0)
        ? Action.TRANSFER
        : Action.UNSHIELD;

    for (let index = 0; index < tx.meta.postBalances.length; index++) {
      const postBalance = new BN(tx.meta.postBalances[index]);
      const preBalance = new BN(tx.meta.preBalances[index]);

      if (postBalance.sub(preBalance).eq(relayerFee)) {
        relayerRecipientSol = accountKeys[index].pubkey;
        break;
      }
    }

    if (type === Action.UNSHIELD) {
      to = accountKeys[1].pubkey;
      from = amountSpl.gt(bn0) ? accountKeys[10].pubkey : accountKeys[9].pubkey;
    }
  } else if (changeSolAmount.gt(bn0) || amountSpl.gt(bn0)) {
    from = accountKeys[0].pubkey;
    to = accountKeys[10].pubkey;
  }

  verifier = accountKeys[2].pubkey;

  const processedTransaction: IndexedTransaction = {
    blockTime: tx.blockTime! * 1000,
    signer: accountKeys[0].pubkey,
    signature,
    accounts: accountKeys.map((item: ParsedMessageAccount | string) =>
      typeof item === "string" ? item : item.pubkey,
    ),
    to,
    from,
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
    message: Buffer.from(message),
  };

  transactions.push(processedTransaction);
}

/**
 * @async
 * @description This functions takes the transactionMeta of  indexer events transactions and extracts relevant data from it
 * @function processIndexerEventsTransactions
 * @param {(ParsedTransactionWithMeta | null)[]} indexerEventsTransactions - An array of indexer event transactions to process
 * @returns {Promise<void>}
 */
// xonoxitron@matteo: flatMaps run faster than forEach
const processIndexerEventsTransactions = async (
  indexerEventsTransactions: (ParsedTransactionWithMeta | null)[],
) => {
  const filteredTransactions = indexerEventsTransactions.filter(
    (tx): tx is ParsedTransactionWithMeta =>
      !!tx &&
      !!tx.meta &&
      !tx.meta.err &&
      !!tx.meta.innerInstructions &&
      tx.meta.innerInstructions.length > 0,
  );

  const indexerTransactionEvents = filteredTransactions.flatMap((tx) =>
    processInnerInstructions(tx),
  );

  return indexerTransactionEvents;
};

const processInnerInstructions = (tx: ParsedTransactionWithMeta) => {
  const innerInstructions = tx.meta!.innerInstructions!;

  const indexerEvents = innerInstructions.flatMap((ix) =>
    ix.instructions?.flatMap((ixInner: any) => processInstruction(tx, ixInner)),
  );

  return indexerEvents.filter(Boolean);
};

const processInstruction = (
  tx: ParsedTransactionWithMeta,
  instruction: any,
) => {
  if (!instruction.data) return [];

  const data = bs58.decode(instruction.data);
  const decodeData = TransactionIndexerEvent.deserialize(data);

  return decodeData ? { ...decodeData, tx } : [];
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
  const signaturesPerRequest = 25;

  let txs = [];
  for (let i = 0; i < signatures.length; i += signaturesPerRequest) {
    const sigsBatch = signatures
      .slice(i, i + signaturesPerRequest)
      .map((sig) => sig.signature);

    const txsBatch = await fetchTransactions(connection, sigsBatch);
    txs.push(...txsBatch);
  }

  const filteredTxs = filterFetchedTransactions(txs);
  const indexerTransactionEvents = await processIndexerEventsTransactions(
    filteredTxs,
  );

  indexerTransactionEvents.forEach((event) => {
    processIndexedTransaction(event, transactions);
  });

  return lastSignature;
};

const fetchTransactions = async (
  connection: Connection,
  sigs: string[],
  maxRetries: number = 3,
  retryDelay: number = 2000,
) => {
  let retries = 0;

  while (retries < maxRetries) {
    try {
      const txsBatch = await connection.getParsedTransactions(sigs, {
        maxSupportedTransactionVersion: 0,
        commitment: "confirmed",
      });

      if (txsBatch.every((t) => t)) {
        return txsBatch;
      } else {
        console.debug("Incomplete transactions batch, retrying...");
      }
    } catch (e) {
      console.error("Error occurred:", e);
      console.debug(`Retry attempt ${retries + 1} of ${maxRetries}`);
    }

    await sleep(retryDelay);
    retries++;
  }

  console.debug(`Max retries reached (${maxRetries}), returning empty array.`);
  return [];
};

const sleep = (ms: number) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};

const filterFetchedTransactions = (txs: any[]) => {
  return txs.filter((tx) => {
    const accountKeys = tx.transaction.message.accountKeys;
    const splNoopIndex = accountKeys.findIndex(
      (item: { pubkey: { toBase58: () => any } }) => {
        const itemStr =
          typeof item === "string" || item instanceof String
            ? item
            : item.pubkey.toBase58();
        return itemStr === new PublicKey(SPL_NOOP_ADDRESS).toBase58();
      },
    );

    return splNoopIndex !== -1;
  });
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

  const getBatchLimit = (index: number) =>
    index === rounds - 1 ? batchOptions.limit! - index * batchSize : batchSize;

  const processBatch = async (index: number) => {
    const batchLimit = getBatchLimit(index);
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
      return null;
    }

    batchBefore = lastSignature.signature;
    await sleep(500);
    return lastSignature;
  };
  // xonoxitron@matteo: Promise.all exploits parallelism
  await Promise.all(Array.from({ length: rounds }, (_, i) => processBatch(i)));

  return transactions.sort(
    (a, b) => a.firstLeafIndex.toNumber() - b.firstLeafIndex.toNumber(),
  );
};
