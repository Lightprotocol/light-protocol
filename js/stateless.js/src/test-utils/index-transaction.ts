import {
  ConfirmedSignatureInfo,
  ConfirmedSignaturesForAddress2Options,
  Connection,
  ParsedMessageAccount,
  ParsedTransactionWithMeta,
  PublicKey,
} from "@solana/web3.js";

import {
  SPL_NOOP_ADDRESS,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { sleep } from "../utils/sleep";
import { accountCompressionProgram } from "../constants";
import { LightSystemProgram } from "../programs/compressed-pda";

type Instruction = {
  accounts: any[];
  data: string;
  programId: PublicKey;
  stackHeight: null | number;
};

import {
  array,
  coption,
  fixedSizeUint8Array,
  u64,
  FixableBeetStruct,
  bignum,
  u8,
} from "@metaplex-foundation/beet";
import { publicKey } from "@metaplex-foundation/beet-solana";
export class ParsingTlvElementBeet {
  constructor(
    readonly discriminator: Uint8Array,
    readonly owner: PublicKey,
    readonly data: number[],
    readonly dataHash: Uint8Array
  ) {}
  static readonly struct = new FixableBeetStruct<
    ParsingTlvElementBeet,
    ParsingTlvElementBeet
  >(
    [
      ["discriminator", fixedSizeUint8Array(8)],
      ["owner", publicKey],
      ["data", fixedSizeUint8Array(8)],
      ["dataHash", fixedSizeUint8Array(8)],
    ],
    (args) =>
      new ParsingTlvElementBeet(
        args.discriminator,
        args.owner,
        args.data,
        args.dataHash
      ),
    "ParsingTlvElementBeet"
  );
}

export class ParsingTlvBeet {
  constructor(readonly tlvElements: number[] | null) {}

  static readonly struct = new FixableBeetStruct<
    ParsingTlvBeet,
    ParsingTlvBeet
  >(
    [["tlvElements", array(u8)]],
    (args) => new ParsingTlvBeet(args.tlvElements),
    "ParsingTlvBeet"
  );
}

export class ParsingUtxoBeet {
  constructor(
    readonly owner: PublicKey,
    readonly blinding: Uint8Array,
    readonly lamports: bignum,
    readonly data: ParsingTlvBeet[] | null
  ) {}

  static readonly struct = new FixableBeetStruct<
    ParsingUtxoBeet,
    ParsingUtxoBeet
  >(
    [
      ["owner", publicKey],
      ["blinding", fixedSizeUint8Array(32)],
      ["lamports", u64],
      ["data", coption(array(ParsingTlvBeet.struct))],
    ],
    (args) =>
      new ParsingUtxoBeet(args.owner, args.blinding, args.lamports, args.data),
    "ParsingUtxo"
  );
}

export class PublicTransactionIndexerEventBeet {
  constructor(
    readonly inUtxos: ParsingUtxoBeet[],
    readonly outUtxos: ParsingUtxoBeet[],
    readonly outUtxoIndices: bignum[],
    readonly deCompressAmount: bignum | null,
    readonly relayFee: bignum | null,
    readonly message: number[] | null
  ) {}

  static readonly struct = new FixableBeetStruct<
    PublicTransactionIndexerEventBeet,
    PublicTransactionIndexerEventBeet
  >(
    [
      ["inUtxos", array(ParsingUtxoBeet.struct)],
      ["outUtxos", array(ParsingUtxoBeet.struct)],
      ["outUtxoIndices", array(u64)],
      ["deCompressAmount", coption(u64)],
      ["relayFee", coption(u64)],
      ["message", coption(array(u8))],
    ],
    (args) =>
      new PublicTransactionIndexerEventBeet(
        args.inUtxos,
        args.outUtxos,
        args.outUtxoIndices,
        args.deCompressAmount,
        args.relayFee,
        args.message
      ),
    "PublicTransactionIndexerEvent"
  );
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

export const findMatchingInstruction = (
  instructions: Instruction[],
  publicKeys: PublicKey[]
): Instruction | undefined => {
  return instructions.find((instruction) =>
    publicKeys.some((pubKey) => pubKey.equals(instruction.programId))
  );
};

// const deserializePrivateEvents = (
//   data: Buffer,
//   tx: ParsedTransactionWithMeta
// ): RpcIndexedTransaction | undefined => {
//   const decodedEvent = new TransactionIndexerEvent().deserialize(data);
//   if (decodedEvent) {
//     decodedEvent["tx"] = tx;
//     return enrichParsedTransactionEvents(decodedEvent);
//   }
// };

/**
 * @description This functions takes the transactionMeta of  indexer events transactions and extracts relevant data from it
 * @function parseTransactionEvents
 * @param {(ParsedTransactionWithMeta | null)[]} indexerEventsTransactions - An array of indexer event transactions to process
 * @returns {Promise<void>}
 */
const parseTransactionEvents = (
  indexerEventsTransactions: (ParsedTransactionWithMeta | null)[],
  transactions: any, //RpcIndexedTransaction[] | PublicTransactionIndexerEventBeet[],
  deserializeFn: any
) => {
  console.log("indexerEventsTransactions", indexerEventsTransactions);
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

        console.log("ixInner before", ixInner);
        const data = bs58.decode(ixInner.data);
        console.log("data", data);

        const decodedEvent = deserializeFn(data, tx);
        console.log("decodedEvent", decodedEvent);
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
  transactions: any; //RpcIndexedTransaction[] | PublicTransactionIndexerEventBeet[];
  deserializeFn: any; //DeserializePublicEvents | DeserializePrivateEvents;
}): Promise<ConfirmedSignatureInfo> {
  const signatures = await connection.getConfirmedSignaturesForAddress2(
    merkleTreeProgramId,
    batchOptions,
    "confirmed"
  );
  console.log("signatures ", signatures.length);

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
        }
      );

      if (!txsBatch.some((t) => !t)) {
        txs = txs.concat(txsBatch);
        index += signaturesPerRequest;
      }
    } catch (e) {
      await sleep(2000);
    }
  }
  console.log("txs ", txs.length);

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

// // More specific function type for deserializing private events
type deserializeTransactionEvents = (
  data: Buffer,
  tx: ParsedTransactionWithMeta
) => PublicTransactionIndexerEventBeet | undefined;

const deserializeTransactionEvents = (data: Buffer) => {
  data = Buffer.from(Array.from(data).map((x: any) => Number(x)));

  try {
    const event = PublicTransactionIndexerEventBeet.struct.deserialize(data)[0];
    return event;
  } catch (e) {
    return null;
  }
};

// export const deserializeTransactionEvents = (data: Buffer) => {
//   console.log(
//     "LightSystemProgram.program.coder",
//     LightSystemProgram.program.coder
//   );
//   const coder = LightSystemProgram.program.coder.types.decode(
//     "PublicTransactionEvent",
//     data
//   );

//   return coder;
// };

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
  transactions?: any[];
}): Promise<{
  transactions: any[];
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
      merkleTreeProgramId: new PublicKey(accountCompressionProgram),
      batchOptions: {
        limit: batchLimit,
        before: batchBefore,
        until: batchOptions.until,
      },
      transactions,
      deserializeFn: deserializeTransactionEvents,
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
        Number(b.outUtxoIndexes[0].toString())
    ),
    oldestFetchedSignature: batchBefore!,
  };
}
