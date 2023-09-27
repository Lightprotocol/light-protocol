import {
  ConfirmedSignatureInfo,
  ConfirmedSignaturesForAddress2Options,
  Connection,
  ParsedMessageAccount,
  ParsedTransactionWithMeta,
  PublicKey,
} from "@solana/web3.js";

import { BN } from "@coral-xyz/anchor";
import * as borsh from "@coral-xyz/borsh";

import { SPL_NOOP_ADDRESS } from "@solana/spl-account-compression";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  VERIFIER_PUBLIC_KEYS,
  MAX_U64,
  FIELD_SIZE,
  merkleTreeProgramId,
  BN_0,
} from "../constants";

import { Action } from "./transaction";

import { getUpdatedSpentUtxos, sleep } from "../utils";
import {
  IndexedTransaction,
  UserIndexedTransaction,
  IndexedTransactionData,
  ParsedIndexedTransaction,
} from "../types";
import { Utxo } from "../utxo";
import { TokenUtxoBalance, Provider } from "../wallet";

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
 *  Call Flow:
 *  fetchRecentTransactions() <-- called in indexer
 *    getTransactionsBatch()
 *      getSigsForAdd()
 *		    getTxForSig()
 *		      make Events:
 *			    parseTransactionEvents()
 *			    enrichParsedTransactionEvents()
 */

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
  indexedTransactions: ParsedIndexedTransaction[],
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
export const findMatchingInstruction = (
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
 * @function enrichParsedTransactionEvents
 * @param {ParsedTransactionWithMeta} tx - The transaction object to process.
 * @param {IndexedTransaction[]} transactions - An array to which the processed transaction data will be pushed.
 * @returns {Promise<void>}
 */
async function enrichParsedTransactionEvents(
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
        amountSpl.eq(BN_0) && amountSol.eq(BN_0)
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
  // 5: relayerRecipientSol
  // 6: senderSol
  // 7: recipientSol
  // 8: tokenProgram
  // 9: tokenAuthority
  // 10: senderSpl
  // 11: recipientSpl
  // 12: registeredVerifierPda
  // 13: logWrapper
  // 14: eventMerkleTree
  let relayerRecipientSol = accountKeys[5];
  let from = accountKeys[6];
  let to = accountKeys[7];
  let fromSpl = accountKeys[10];
  let toSpl = accountKeys[11];

  const nullifiers = event.nullifiers;

  let solTokenPoolIndex = type === Action.SHIELD ? 9 : 8;
  let changeSolAmount = new BN(
    tx.meta.postBalances[solTokenPoolIndex] -
      tx.meta.preBalances[solTokenPoolIndex],
  );
  changeSolAmount = changeSolAmount.lt(BN_0)
    ? changeSolAmount.abs().sub(relayerFee)
    : changeSolAmount;

  transactions.push({
    blockTime: tx.blockTime! * 1000,
    signer: accountKeys[0],
    signature,
    to,
    from,
    //TODO: check if this is the correct type after latest main?
    //@ts-ignore
    toSpl,
    fromSpl,
    verifier,
    relayerRecipientSol,
    type,
    changeSolAmount: changeSolAmount.toString("hex"),
    publicAmountSol: amountSol.toString("hex"),
    publicAmountSpl: amountSpl.toString("hex"),
    encryptedUtxos,
    leaves,
    nullifiers,
    relayerFee: relayerFee.toString("hex"),
    firstLeafIndex: firstLeafIndex.toString("hex"),
    message: Buffer.from(message),
  });
}

/**
 * @async
 * @description This functions takes the transactionMeta of  indexer events transactions and extracts relevant data from it
 * @function parseTransactionEvents
 * @param {(ParsedTransactionWithMeta | null)[]} indexerEventsTransactions - An array of indexer event transactions to process
 * @returns {Promise<void>}
 */
const parseTransactionEvents = (
  indexerEventsTransactions: (ParsedTransactionWithMeta | null)[],
) => {
  const parsedTransactionEvents: IndexedTransactionData[] = [];

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
          parsedTransactionEvents.push({
            ...decodeData,
            tx,
          });
        }
      });
    });
  });

  return parsedTransactionEvents;
};

/**
 * @description Fetches transactions for the specified merkleTreeProgramId in batches
 * and process the incoming transaction using the enrichParsedTransactionEvents.
 * This function will handle retries and sleep to prevent rate-limiting issues.
 * @param {Connection} connection - The Connection object to interact with the Solana network.
 * @param {PublicKey} merkleTreeProgramId - The PublicKey of the Merkle tree program.
 * @param {ConfirmedSignaturesForAddress2Options} batchOptions - Options for fetching transaction batches,
 * including starting transaction signature (after), ending transaction signature (before), and batch size (limit).
 * @param {any[]} transactions - The array where the fetched transactions will be stored.
 * @returns {Promise<string>} - The signature of the last fetched transaction.
 */
// TODO: consider explicitly returning a new txs array instead of mutating the passed in oneDasync functio getn
async function getTransactionsBatch({
  connection,
  merkleTreeProgramId,
  batchOptions,
  transactions,
}: {
  connection: Connection;
  merkleTreeProgramId: PublicKey;
  batchOptions: ConfirmedSignaturesForAddress2Options;
  transactions: any;
}): Promise<ConfirmedSignatureInfo> {
  const signatures = await connection.getConfirmedSignaturesForAddress2(
    new PublicKey(merkleTreeProgramId),
    batchOptions,
    "confirmed",
  );
  const lastSignature = signatures[signatures.length - 1];
  let txs: (ParsedTransactionWithMeta | null)[] = [];
  let index = 0;
  const signaturesPerRequest = 5;

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

  const transactionEvents = txs.filter((tx: any) => {
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

  const parsedTransactionEvents = parseTransactionEvents(transactionEvents);
  parsedTransactionEvents.forEach((event) => {
    enrichParsedTransactionEvents(event!, transactions);
  });
  return lastSignature;
}

/**
 * @description Fetches recent transactions for the specified merkleTreeProgramId.
 * This function will call getTransactionsBatch multiple times to fetch transactions in batches.
 * @param {Connection} connection - The Connection object to interact with the Solana network.
 * @param {ConfirmedSignaturesForAddress2Options} batchOptions - Options for fetching transaction batches,
 * including starting transaction signature (after), ending transaction signature (before), and batch size (limit).
 * @param {boolean} dedupe=false - Whether to deduplicate transactions or not.
 * @returns {Promise<indexedTransaction[]>} Array of indexedTransactions
 */

export async function fetchRecentTransactions({
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
  transactions?: IndexedTransaction[];
}): Promise<{
  transactions: IndexedTransaction[];
  oldestFetchedSignature: string;
}> {
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
  return {
    transactions: transactions.sort(
      (a, b) =>
        new BN(a.firstLeafIndex, "hex").toNumber() -
        new BN(b.firstLeafIndex, "hex").toNumber(),
    ),
    oldestFetchedSignature: batchBefore!,
  };
}
