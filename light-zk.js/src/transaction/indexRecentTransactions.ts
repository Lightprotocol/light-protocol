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
  VERIFIER_PUBLIC_KEYS,
  MAX_U64,
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
    borsh.vecU8("message"),
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
      trx.signer.toBase58() === provider.wallet.publicKey.toBase58();

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

type Instruction = {
  accounts: any[];
  data: string;
  programId: PublicKey;
  stackHeight: null | number;
};
const findMatchingInstruction = (
  instructions: Instruction[],
  publicKeys: PublicKey[],
): Instruction | undefined => {
  return instructions.find((instruction) =>
    publicKeys.some((pubKey) => pubKey.equals(instruction.programId)),
  );
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
    message,
  } = event;
  if (!tx || !tx.meta || tx.meta.err) return;

  // check first whether we can find an instruction to a verifier program in the main instructions
  let instruction = findMatchingInstruction(
    tx.transaction.message.instructions,
    VERIFIER_PUBLIC_KEYS,
  );
  // if we didn't find a main instruction to a verifier program we check the inner instructions
  // this is the case for private programs which call verifier two via cpi
  for (let innerInstruction of tx.meta.innerInstructions) {
    if (!instruction)
      instruction = findMatchingInstruction(
        innerInstruction.instructions,
        VERIFIER_PUBLIC_KEYS,
      );
  }
  if (!instruction) return;

  const signature = tx.transaction.signatures[0];
  let accountKeys = instruction.accounts;
  let verifier = instruction.programId;

  const getTypeAndAmounts = (
    publicAmountSpl: Uint8Array,
    publicAmountSol: Uint8Array,
  ) => {
    let type = Action.SHIELD;
    let amountSpl = new BN(publicAmountSpl, 32, "be");
    let amountSol = new BN(publicAmountSol, 32, "be");

    let splIsU64 = amountSpl.lte(MAX_U64);
    let solIsU64 = amountSol.lte(MAX_U64);
    if (!splIsU64 || !solIsU64) {
      amountSpl = amountSpl.sub(FIELD_SIZE).mod(FIELD_SIZE).abs();
      amountSol = amountSol
        .sub(FIELD_SIZE)
        .mod(FIELD_SIZE)
        .abs()
        .sub(relayerFee);
      type =
        amountSpl.eq(new BN(0)) && amountSol.eq(new BN(0))
          ? Action.TRANSFER
          : Action.UNSHIELD;
    }
    return { amountSpl, amountSol, type };
  };

  const { type, amountSpl, amountSol } = getTypeAndAmounts(
    publicAmountSpl,
    publicAmountSol,
  );
  const convertToPublicKey = (key: PublicKey | string): PublicKey => {
    return key instanceof PublicKey ? key : new PublicKey(key);
  };
  accountKeys = accountKeys.map((key) => convertToPublicKey(key));
  // 0: signingAddress
  // 1: systemProgram
  // 2: programMerkleTree
  // 3: transactionMerkleTree
  // 4: authority
  // 5: tokenProgram
  // 6: senderSpl
  // 7: recipientSpl
  // 8: senderSol
  // 9: recipientSol
  // 10: relayerRecipientSol
  // 11: tokenAuthority
  // 12: registeredVerifierPda
  // 13: logWrapper
  let fromSpl = accountKeys[6];
  let toSpl = accountKeys[7];
  let from = accountKeys[8];
  let to = accountKeys[9];
  let relayerRecipientSol = accountKeys[10];

  const nullifiers = event.nullifiers;

  let solTokenPoolIndex = type === Action.SHIELD ? 9 : 8;
  let changeSolAmount = new BN(
    tx.meta.postBalances[solTokenPoolIndex] -
      tx.meta.preBalances[solTokenPoolIndex],
  );
  changeSolAmount = changeSolAmount.lt(new BN(0))
    ? changeSolAmount.abs().sub(relayerFee)
    : changeSolAmount;

  transactions.push({
    blockTime: tx.blockTime! * 1000,
    signer: accountKeys[0],
    signature,
    accounts: accountKeys,
    to,
    from,
    toSpl,
    fromSpl,
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
  });
}

/**
 * @async
 * @description This functions takes the transactionMeta of  indexer events transactions and extracts relevant data from it
 * @function processIndexerEventsTransactions
 * @param {(ParsedTransactionWithMeta | null)[]} indexerEventsTransactions - An array of indexer event transactions to process
 * @returns {Promise<void>}
 */
const processIndexerEventsTransactions = (
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
  const signaturesPerRequest = 25;

  while (index < signatures.length) {
    try {
      const txsBatch = await connection.getParsedTransactions(
        signatures
          .slice(index, index + signaturesPerRequest)
          .map((sig) => sig.signature),
        {
          maxSupportedTransactionVersion: 0,
          commitment: "confirmed",
        },
      );

      if (!txsBatch.some((t) => !t)) {
        txs = txs.concat(txsBatch);
        index += signaturesPerRequest;
      }
    } catch (e) {
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

  const indexerTransactionEvents = processIndexerEventsTransactions(
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
  transactions = [],
}: {
  connection: Connection;
  batchOptions: ConfirmedSignaturesForAddress2Options;
  dedupe?: boolean;
  transactions?: IndexedTransaction[];
}): Promise<IndexedTransaction[]> => {
  const batchSize = 1000;
  const rounds = Math.ceil(batchOptions.limit! / batchSize);

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
