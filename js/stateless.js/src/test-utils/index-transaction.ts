import {
  ConfirmedSignatureInfo,
  ConfirmedSignaturesForAddress2Options,
  Connection,
  ParsedMessageAccount,
  ParsedTransactionWithMeta,
  PublicKey,
} from '@solana/web3.js';

import {
  SPL_NOOP_ADDRESS,
  SPL_NOOP_PROGRAM_ID,
} from '@solana/spl-account-compression';
import { bs58 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { sleep } from '../utils';
import { accountCompressionProgram } from '../constants';

type Instruction = {
  accounts: any[];
  data: string;
  programId: PublicKey;
  stackHeight: null | number;
};

import { LightSystemProgram } from '../programs';

/**
 * TODO: simplify this.
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
  publicKeys: PublicKey[],
): Instruction | undefined => {
  return instructions.find((instruction) =>
    publicKeys.some((pubKey) => pubKey.equals(instruction.programId)),
  );
};

const parseTransactionEvents = (
  indexerEventsTransactions: (ParsedTransactionWithMeta | null)[],
  transactions: any, //RpcIndexedTransaction[] | PublicTransactionIndexerEventBeet[],
  deserializeFn: any,
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

    /// TODO: make robust
    tx.meta.innerInstructions.forEach((ix) => {
      ix.instructions.forEach((ixInner: any) => {
        if (!ixInner.data) return;
        if (ixInner.programId.toBase58() !== SPL_NOOP_PROGRAM_ID.toBase58())
          return;

        const data = bs58.decode(ixInner.data);
        if (data.length < 800) return;
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
  transactions: any; //RpcIndexedTransaction[] | PublicTransactionIndexerEventBeet[];
  deserializeFn: any; //DeserializePublicEvents | DeserializePrivateEvents;
}): Promise<ConfirmedSignatureInfo> {
  const signatures = await connection.getConfirmedSignaturesForAddress2(
    merkleTreeProgramId,
    batchOptions,
    'confirmed',
  );
  console.log('signatures ', signatures.length);

  const lastSignature = signatures[signatures.length - 1];
  let txs: (ParsedTransactionWithMeta | null)[] = [];
  let index = 0;
  const signaturesPerRequest = 5;

  while (index < signatures.length) {
    console.log(
      'while index <sigs i, sigs',
      index,
      signatures.length,
      signatures[0].signature,
    );
    try {
      let txsBatch: any = await connection.getParsedTransactions(
        signatures
          .slice(index, index + signaturesPerRequest)
          .map((sig) => sig.signature),
        {
          maxSupportedTransactionVersion: 0,
          commitment: 'confirmed',
        },
      );
      if (!txsBatch.some((t: ParsedTransactionWithMeta) => !t)) {
        txs = txs.concat(txsBatch);
        index += signaturesPerRequest;
      }
    } catch (e) {
      console.log('error fetching txs', e);
      await sleep(2000);
    }
  }
  console.log('txs ', txs.length);

  const transactionEvents = txs.filter((tx: any) => {
    const accountKeys = tx.transaction.message.accountKeys;
    const splNoopIndex = accountKeys.findIndex((item: ParsedMessageAccount) => {
      const itemStr =
        typeof item === 'string' || item instanceof String
          ? item
          : item.pubkey.toBase58();
      return itemStr === new PublicKey(SPL_NOOP_ADDRESS).toBase58();
    });

    if (splNoopIndex) {
      return txs;
    }
  });
  parseTransactionEvents(transactionEvents, transactions, deserializeFn);
  console.log('no des');
  return lastSignature;
}

// TODO: wrap up testing
// // More specific function type for deserializing private events
// type deserializeTransactionEvents = (
//   data: Buffer,
//   tx: ParsedTransactionWithMeta,
// ) => PublicTransactionIndexerEventBeet | undefined;

// const deserializeTransactionEvents = (data: Buffer) => {
//   data = Buffer.from(Array.from(data).map((x: any) => Number(x)));

//   try {
//     const event = PublicTransactionIndexerEventBeet.struct.deserialize(data)[0];
//     return event;
//   } catch (e) {
//     console.log('couldnt deserializing event', e);
//     return null;
//   }
// };

// TODO: rm TEMP for debugging utxo.data serde
const deserializeTransactionEventsTokenAnchor = (data: Buffer) => {
  data = Buffer.from(data);
  console.log('data from chain', Array.from(data).toString());
  const discBuffer = Buffer.alloc(8, 1); // Creates an 8-byte buffer filled with ones
  try {
    const event = LightSystemProgram.program.coder.types.decode(
      'PublicTransactionEvent',
      Buffer.concat([discBuffer, data]),
    );
    return event;
  } catch (e) {
    console.log('couldnt deserializing event', e);
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
    console.log('batchlimit=?', batchLimit, 'round', i);
    const lastSignature = await getTransactionsBatch({
      connection,
      merkleTreeProgramId: new PublicKey(accountCompressionProgram),
      batchOptions: {
        limit: batchLimit,
        before: batchBefore,
        until: batchOptions.until,
      },
      transactions,
      // TODO: rm debug
      // deserializeFn: deserializeTransactionEvents,
      deserializeFn: deserializeTransactionEventsTokenAnchor,
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
        Number(a.outUtxoIndices[0].toString()) -
        Number(b.outUtxoIndices[0].toString()),
    ),
    oldestFetchedSignature: batchBefore!,
  };
}
