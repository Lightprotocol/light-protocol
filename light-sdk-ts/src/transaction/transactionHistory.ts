import {
  Connection,
  ParsedTransactionWithMeta,
  PublicKey,
} from "@solana/web3.js";
import {
  merkleTreeProgramId,
  FIELD_SIZE,
  REGISTERED_POOL_PDA_SOL,
} from "../constants";
import { sleep } from "../utils";
import { IDL_VERIFIER_PROGRAM_ZERO } from "../idls";
import { BorshCoder, BN } from "@coral-xyz/anchor";

// TODO: to and from in the all transaction
// TODO: Manage functions to fetch all transactions
// TODO: refactor
// TODO: file structure and put inside the testRelayer
// TODO: test-cases for transaction

interface Data {
  publicAmountSpl: Uint8Array;
  publicAmountSol: Uint8Array;
  leaves: BN[];
  encryptedUtxos: any[];
  nullifiers: any[];
  relayerFee: BN;
}

interface DecodedData {
  data: Data;
}

async function processTransaction(
  tx: ParsedTransactionWithMeta,
  transactions: any[],
): Promise<void> {
  if (!tx || !tx.meta || tx.meta.err) return;

  const signature = tx.transaction.signatures[0];
  const tokenPool = new PublicKey(REGISTERED_POOL_PDA_SOL);
  const accountKeys = tx.transaction.message.accountKeys;
  const i = accountKeys.findIndex((item: any) => {
    const itemStr =
      typeof item === "string" || item instanceof String
        ? item
        : item.pubkey.toBase58();
    return itemStr === tokenPool.toBase58();
  });

  const amount = tx.meta.postBalances[i] - tx.meta.preBalances[i];
  let from: PublicKey;
  let to: PublicKey = accountKeys[2].pubkey;
  let type: string;

  const coder = new BorshCoder(IDL_VERIFIER_PROGRAM_ZERO);

  if (amount < 0 && !tx.meta.err) {
    from = new PublicKey(REGISTERED_POOL_PDA_SOL);
    type =
      tx.transaction.message.accountKeys.length <= 16 && i === 10
        ? "transfer"
        : "unshield";

    const instructions = tx.transaction.message.instructions;
    for (const instruction of instructions) {
      // @ts-ignore
      const rawData = instruction.data;
      const data = coder.instruction.decode(
        rawData,
        "base58",
      ) as DecodedData | null;

      if (data) {
        const amountSpl =
          parseInt(
            new BN(data.data["publicAmountSpl"])
              .sub(FIELD_SIZE)
              .mod(FIELD_SIZE)
              .toString(),
          ) * -1;

        const amountSol =
          parseInt(
            new BN(data.data["publicAmountSol"])
              .sub(FIELD_SIZE)
              .mod(FIELD_SIZE)
              .toString(),
          ) * -1;

        const relayerFee = data.data["relayerFee"].toString();
        const commitment = new BN(data.data["leaves"][0]).toString();

        const toIndex = tx.meta.postBalances.findIndex(
          (el: any, index: any) => {
            return (
              tx.meta!.postBalances[index] - tx.meta!.preBalances[index] ===
              amount * -1 - parseInt(relayerFee)
            );
          },
        );

        if (toIndex > 0) {
          to = accountKeys[toIndex].pubkey;
        }

        transactions.push({
          blockTime: tx.blockTime! * 1000,
          signer: accountKeys[0].pubkey,
          signature,
          accounts: accountKeys,
          to,
          from: from.toBase58(),
          type,
          amount: amount * -1 - parseInt(relayerFee),
          amountSol,
          amountSpl,
          commitment,
          encryptedUtxos: data.data["encryptedUtxos"],
          leaves: data.data["leaves"],
          nullifiers: data.data["nullifiers"],
          relayerFee,
        });
      }
    }
  } else if ((amount > 0 && !tx.meta.err && i === 10) || i === 11) {
    from = accountKeys[0].pubkey;
    to = new PublicKey(REGISTERED_POOL_PDA_SOL);
    type = "shield";

    const instructions = tx.transaction.message.instructions;
    for (const instruction of instructions) {
      // @ts-ignore
      const rawData = instruction.data;
      const data = coder.instruction.decode(
        rawData,
        "base58",
      ) as DecodedData | null;

      if (data) {
        const amountSpl = new BN(
          data.data["publicAmountSpl"].slice(24, 32),
        ).toString();
        const amountSol = new BN(
          data.data["publicAmountSol"].slice(24, 32),
        ).toString();
        const commitment = new BN(data.data["leaves"][0]).toString();

        transactions.push({
          blockTime: tx.blockTime! * 1000,
          signer: accountKeys[0].pubkey,
          signature,
          accounts: accountKeys,
          to: to.toBase58(),
          from: from.toBase58(),
          type,
          amount,
          amountSol,
          amountSpl,
          commitment,
          encryptedUtxos: data.data["encryptedUtxos"],
          leaves: data.data["leaves"],
          nullifiers: data.data["nullifiers"],
          relayerFee: data.data["relayerFee"].toString(),
        });
      }
    }
  }
}

type BatchOptions = {
  limit: number;
  before: any;
  until: any;
};

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
}) => {
  const batchSize = 1000;
  const rounds = Math.ceil(limit / batchSize);
  const transactions: any[] = [];

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

    batchBefore = lastSignature.signature;
    await sleep(1000);
  }

  // Optionally deduplicate transactions
  // ...

  return transactions.sort((a, b) => b.blockTime - a.blockTime);
};
