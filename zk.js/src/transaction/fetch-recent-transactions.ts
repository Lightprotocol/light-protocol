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
import {
  SPL_NOOP_ADDRESS,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  UserIndexedTransaction,
  IndexedTransactionData,
  ParsedIndexedTransaction,
  RpcIndexedTransaction,
  Action,
} from "../types";
import {
  VERIFIER_PUBLIC_KEYS,
  MAX_U64,
  FIELD_SIZE,
  merkleTreeProgramId,
  BN_0,
  MERKLE_TREE_SET,
} from "../constants";
import { getUpdatedSpentUtxos, sleep } from "../utils";
import { TokenUtxoBalance } from "../build-balance";
import { Provider } from "../provider";
import { PlaceHolderTData, ProgramUtxo, Utxo } from "../utxo";
import { getIdsFromEncryptedUtxos } from "../test-utils";
import {
  array,
  coption,
  fixedSizeUint8Array,
  u64,
  uniformFixedSizeArray,
  FixableBeetStruct,
  bignum,
  u8,
} from "@metaplex-foundation/beet";
import { publicKey } from "@metaplex-foundation/beet-solana";

export class TransactionIndexerEvent {
  borshSchema = borsh.struct([
    borsh.vec(borsh.array(borsh.u8(), 32), "leaves"),
    borsh.array(borsh.u8(), 32, "publicAmountSpl"),
    borsh.array(borsh.u8(), 32, "publicAmountSol"),
    borsh.u64("rpcFee"),
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

// Define ParsingUtxo TypeScript class and Beet struct
export class ParsingUtxoBeet {
  constructor(
    readonly version: bignum,
    readonly poolType: bignum,
    readonly amounts: bignum[],
    readonly splAssetMint: PublicKey,
    readonly owner: Uint8Array,
    readonly blinding: Uint8Array,
    readonly dataHash: Uint8Array,
    readonly metaHash: Uint8Array,
    readonly address: Uint8Array,
    readonly message: number[] | null,
  ) {}

  static readonly struct = new FixableBeetStruct<
    ParsingUtxoBeet,
    ParsingUtxoBeet
  >(
    [
      ["version", u64],
      ["poolType", u64],
      ["amounts", uniformFixedSizeArray(u64, 2)],
      ["splAssetMint", coption(publicKey)],
      ["owner", fixedSizeUint8Array(32)],
      ["blinding", fixedSizeUint8Array(32)],
      ["dataHash", fixedSizeUint8Array(32)],
      ["metaHash", fixedSizeUint8Array(32)],
      ["address", fixedSizeUint8Array(32)],
      ["message", coption(array(u8))],
    ],
    (args) =>
      new ParsingUtxoBeet(
        args.version,
        args.poolType,
        args.amounts,
        args.splAssetMint,
        args.owner,
        args.blinding,
        args.dataHash,
        args.metaHash,
        args.address,
        args.message,
      ),
    "ParsingUtxo",
  );
}

export class PublicTransactionIndexerEventBeet {
  constructor(
    readonly inUtxoHashes: Uint8Array[],
    readonly outUtxos: ParsingUtxoBeet[],
    readonly outUtxoIndexes: bignum[],
    readonly publicAmountSol: Uint8Array | null,
    readonly publicAmountSpl: Uint8Array | null,
    readonly rpcFee: bignum | null,
    readonly message: number[] | null,
    readonly transactionHash: Uint8Array | null,
    readonly programId: PublicKey | null,
  ) {}

  static readonly struct = new FixableBeetStruct<
    PublicTransactionIndexerEventBeet,
    PublicTransactionIndexerEventBeet
  >(
    [
      ["inUtxoHashes", array(fixedSizeUint8Array(32))],
      ["outUtxos", array(ParsingUtxoBeet.struct)],
      ["outUtxoIndexes", array(u64)],
      ["publicAmountSol", coption(fixedSizeUint8Array(32))],
      ["publicAmountSpl", coption(fixedSizeUint8Array(32))],
      ["rpcFee", coption(u64)],
      ["message", coption(array(u8))],
      ["transactionHash", coption(fixedSizeUint8Array(32))],
      ["programId", coption(publicKey)],
    ],
    (args) =>
      new PublicTransactionIndexerEventBeet(
        args.inUtxoHashes,
        args.outUtxos,
        args.outUtxoIndexes,
        args.publicAmountSol,
        args.publicAmountSpl,
        args.rpcFee,
        args.message,
        args.transactionHash,
        args.programId,
      ),
    "PublicTransactionIndexerEvent",
  );
}

// not used
export class PublicTransactionIndexerEventAnchor {
  utxo = borsh.struct([
    borsh.u64("version"),
    borsh.u64("poolType"),
    borsh.array(borsh.u64(), 2, "amounts"),
    borsh.publicKey("splAssetMint"),
    borsh.array(borsh.u8(), 32, "owner"),
    borsh.array(borsh.u8(), 32, "blinding"),
    borsh.array(borsh.u8(), 32, "dataHash"),
    borsh.array(borsh.u8(), 32, "metaHash"),
    borsh.array(borsh.u8(), 32, "address"),
    borsh.option(borsh.vecU8(), "message"),
  ]);

  borshSchema = borsh.struct([
    borsh.vec(borsh.array(borsh.u8(), 32), "inUtxohashes"),
    borsh.vec(borsh.vecU8(), "outUtxos"),
    borsh.vec(borsh.u64(), "outUtxoIndexes"),
    borsh.option(borsh.array(borsh.u8(), 32), "publicAmountSpl"),
    borsh.option(borsh.array(borsh.u8(), 32), "publicAmountSol"),
    borsh.option(borsh.u64(), "rpcFee"),
    borsh.option(borsh.vecU8(), "message"),
    borsh.option(borsh.array(borsh.u8(), 32), "transactionHash"),
    borsh.option(borsh.publicKey(), "programId"),
  ]);

  deserialize(buffer: Buffer): any | null {
    try {
      const _internal = this.borshSchema.decode(buffer);
      _internal.outUtxos = _internal.outUtxos.map((utxo: any) => {
        return this.utxo.decode(utxo);
      });
      return _internal;
    } catch (e) {
      return null;
    }
  }

  deserializeUtxo(buffer: Buffer): any | null {
    try {
      return this.utxo.decode(buffer);
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
    const nullifierZero = new BN(trx.nullifiers[0]);

    const nullifierOne = new BN(trx.nullifiers[1]);

    const isFromUser = trx.signer === provider.wallet.publicKey.toBase58();

    const inSpentUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[] = [];
    const outSpentUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[] = [];

    spentUtxos?.forEach((sUtxo) => {
      const matchesNullifier =
        sUtxo.nullifier.eq(nullifierOne) || sUtxo.nullifier.eq(nullifierZero);

      let matchesCommitment = false;
      for (const leaf of trx.leaves) {
        if (!matchesCommitment) {
          matchesCommitment = sUtxo.hash.eq(new BN(leaf, "le"));
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
function enrichParsedTransactionEvents(event: IndexedTransactionData) {
  // check if transaction contains the meta data or not , else return without processing transaction
  const {
    tx,
    publicAmountSol,
    publicAmountSpl,
    rpcFee,
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
  for (const innerInstruction of tx.meta.innerInstructions) {
    if (!instruction)
      instruction = findMatchingInstruction(
        innerInstruction.instructions,
        VERIFIER_PUBLIC_KEYS,
      );
  }
  if (!instruction) return;

  const signature = tx.transaction.signatures[0];
  let accountKeys = instruction.accounts;
  const verifier = instruction.programId;

  const getTypeAndAmounts = (
    publicAmountSpl: Uint8Array,
    publicAmountSol: Uint8Array,
  ) => {
    let type = Action.COMPRESS;
    let amountSpl = new BN(publicAmountSpl, 32, "be");
    let amountSol = new BN(publicAmountSol, 32, "be");

    const splIsU64 = amountSpl.lte(MAX_U64);
    const solIsU64 = amountSol.lte(MAX_U64);
    if (!splIsU64 || !solIsU64) {
      amountSpl = amountSpl.sub(FIELD_SIZE).mod(FIELD_SIZE).abs();
      amountSol = amountSol.sub(FIELD_SIZE).mod(FIELD_SIZE).abs().sub(rpcFee);
      type =
        amountSpl.eq(BN_0) && amountSol.eq(BN_0)
          ? Action.TRANSFER
          : Action.DECOMPRESS;
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
  // 5: rpcRecipientSol
  // 6: senderSol
  // 7: recipientSol
  // 8: tokenProgram
  // 9: tokenAuthority
  // 10: senderSpl
  // 11: recipientSpl
  // 12: registeredVerifierPda
  // 13: logWrapper
  // 14: eventMerkleTree
  const rpcRecipientSol = accountKeys[5];
  const from = accountKeys[6];
  const to = accountKeys[7];
  const fromSpl = accountKeys[10];
  const toSpl = accountKeys[11];

  const nullifiers = event.nullifiers;

  const solTokenPoolIndex = type === Action.COMPRESS ? 9 : 8;
  let changeSolAmount = new BN(
    tx.meta.postBalances[solTokenPoolIndex] -
      tx.meta.preBalances[solTokenPoolIndex],
  );
  changeSolAmount = changeSolAmount.lt(BN_0)
    ? changeSolAmount.abs().sub(rpcFee)
    : changeSolAmount;
  const IDs = getIdsFromEncryptedUtxos(
    Buffer.from(encryptedUtxos),
    leaves.length,
  );
  // transactions.push({
  //   IDs,
  //   merkleTreePublicKey:
  //     MerkleTreeConfig.getTransactionMerkleTreePda().toBase58(),
  //   transaction: {
  //     blockTime: tx.blockTime! * 1000,
  //     signer: accountKeys[0],
  //     signature,
  //     to,
  //     from,
  //     //TODO: check if this is the correct type after latest main?
  //     //@ts-ignore
  //     toSpl,
  //     fromSpl,
  //     verifier: verifier.toBase58(),
  //     rpcRecipientSol,
  //     type,
  //     changeSolAmount: changeSolAmount.toString("hex"),
  //     publicAmountSol: amountSol.toString("hex"),
  //     publicAmountSpl: amountSpl.toString("hex"),
  //     encryptedUtxos: encryptedUtxos,
  //     leaves,
  //     nullifiers,
  //     rpcFee: rpcFee.toString("hex"),
  //     firstLeafIndex: firstLeafIndex.toString("hex"),
  //     message: message,
  //   },
  // });
  return {
    IDs,
    merkleTreePublicKey: MERKLE_TREE_SET.toBase58(),
    transaction: {
      blockTime: tx.blockTime! * 1000,
      signer: accountKeys[0],
      signature,
      to,
      from,
      //TODO: check if this is the correct type after latest main?
      //@ts-ignore
      toSpl,
      fromSpl,
      verifier: verifier.toBase58(),
      rpcRecipientSol,
      type,
      changeSolAmount: changeSolAmount.toString("hex"),
      publicAmountSol: amountSol.toString("hex"),
      publicAmountSpl: amountSpl.toString("hex"),
      encryptedUtxos: encryptedUtxos,
      leaves,
      nullifiers,
      rpcFee: rpcFee.toString("hex"),
      firstLeafIndex: firstLeafIndex.toString("hex"),
      message: message,
    },
  };
}

const deserializePrivateEvents = (
  data: Buffer,
  tx: ParsedTransactionWithMeta,
): RpcIndexedTransaction | undefined => {
  const decodedEvent = new TransactionIndexerEvent().deserialize(data);
  decodedEvent["tx"] = tx;
  if (decodedEvent) {
    return enrichParsedTransactionEvents(decodedEvent);
  }
};

/**
 * @async
 * @description This functions takes the transactionMeta of  indexer events transactions and extracts relevant data from it
 * @function parseTransactionEvents
 * @param {(ParsedTransactionWithMeta | null)[]} indexerEventsTransactions - An array of indexer event transactions to process
 * @returns {Promise<void>}
 */
const parseTransactionEvents = (
  indexerEventsTransactions: (ParsedTransactionWithMeta | null)[],
  transactions: RpcIndexedTransaction[] | PublicTransactionIndexerEventBeet[],
  deserializeFn: DeserializePublicEvents | DeserializePrivateEvents,
) => {
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
        if (ixInner.programId.toBase58() !== SPL_NOOP_PROGRAM_ID.toBase58())
          return;
        const data = bs58.decode(ixInner.data);

        const decodedEvent = deserializeFn(data, tx);

        if (decodedEvent) {
          transactions.push(decodedEvent);
        }
      });
    });
  });
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
// TODO: consider explicitly returning a new txs array instead of mutating the passed in one.
async function getTransactionsBatch({
  connection,
  merkleTreeProgramId,
  batchOptions,
  transactions,
  deserializeFn,
}: {
  connection: Connection;
  merkleTreeProgramId: PublicKey;
  batchOptions: ConfirmedSignaturesForAddress2Options;
  transactions: RpcIndexedTransaction[] | PublicTransactionIndexerEventBeet[];
  deserializeFn: DeserializePublicEvents | DeserializePrivateEvents;
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
  parseTransactionEvents(transactionEvents, transactions, deserializeFn);
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
  transactions?: RpcIndexedTransaction[];
}): Promise<{
  transactions: RpcIndexedTransaction[];
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
      deserializeFn: deserializePrivateEvents,
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
        // @ts-ignore unwarranted error because of mixed types
        new BN(a.transaction.firstLeafIndex, "hex").toNumber() -
        // @ts-ignore unwarranted error because of mixed types
        new BN(b.transaction.firstLeafIndex, "hex").toNumber(),
    ),
    oldestFetchedSignature: batchBefore!,
  };
}
type DeserializePublicEvents = (data: Buffer) => any | null;

// More specific function type for deserializing private events
type DeserializePrivateEvents = (
  data: Buffer,
  tx: ParsedTransactionWithMeta,
) => RpcIndexedTransaction | undefined;
const deserializePublicEvents = (data: Buffer) => {
  data = Buffer.from(Array.from(data).map((x: any) => Number(x)));

  try {
    const event = PublicTransactionIndexerEventBeet.struct.deserialize(data)[0];
    return event;
  } catch (e) {
    return null;
  }
};

export async function fetchRecentPublicTransactions({
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
  transactions?: PublicTransactionIndexerEventBeet[];
}): Promise<{
  transactions: PublicTransactionIndexerEventBeet[];
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
      deserializeFn: deserializePublicEvents,
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
        Number(a.outUtxoIndexes[0].toString()) -
        Number(b.outUtxoIndexes[0].toString()),
    ),
    oldestFetchedSignature: batchBefore!,
  };
}
