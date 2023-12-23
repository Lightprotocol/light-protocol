import {
  AccountMeta,
  PublicKey,
  TransactionInstruction,
  VersionedTransaction,
} from "@solana/web3.js";
import {
  SignaturesWithBlockhashInfo,
  createSolanaTransactions,
} from "@lightprotocol/zk.js";
import { getLightProvider, getRelayer } from "../../utils/provider";
import { relayQueue } from "../../db/redis";
import { sha3_256 } from "@noble/hashes/sha3";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

export function getUidFromIxs(ixs: TransactionInstruction[]): string {
  const hasher = sha3_256.create();
  ixs.forEach((ix) => {
    hasher.update(new Uint8Array([...ix.data]));
  });
  return bs58.encode(hasher.digest());
}

/**
 * Creates Transactions from instruction payload, serializes them and adds relay job to queue. Returns signatures and blockhashInfo.
 * Note: assumes the queue picks up the job ASAP and optimistically signs the transactions.
 */
async function addRelayJob({
  instructions,
}: {
  instructions: TransactionInstruction[];
}) {
  const uid = getUidFromIxs(instructions); // TODO: add a test that checks that this is unique
  /// TODO: this is inefficient
  const provider = await getLightProvider();

  const versionedTransactionLookupTableAccountArgs =
    await provider.getVersionedTransactionLookupTableAccountArgs();

  const { blockhash, lastValidBlockHeight } =
    await provider.connection!.getLatestBlockhash();

  const transactions = createSolanaTransactions(
    instructions,
    blockhash,
    versionedTransactionLookupTableAccountArgs,
  );
  const signedTransactions: VersionedTransaction[] =
    await provider.wallet.signAllTransactions(transactions);

  const serializedTransactions = signedTransactions.map((tx) => {
    return Buffer.from(tx.serialize()).toString("base64");
  });

  const job = await relayQueue.add(
    "relay",
    {
      signedTransactions: serializedTransactions,
      blockhashInfo: { lastValidBlockHeight, blockhash },
      response: null,
    },
    { jobId: uid },
  );

  return {
    job,
    signatures: signedTransactions.map((tx) => bs58.encode(tx.signatures[0])),
    blockhashInfo: { lastValidBlockHeight, blockhash },
  };
}

function validateReqParams(req: any) {
  if (!req.body.instructions) throw new Error("No instructions provided");
  if (!req.body.instructions.length)
    throw new Error("No instructions provided");
  if (!req.body.instructions[0].keys)
    throw new Error("No keys provided in instructions");
  if (!req.body.instructions[0].programId)
    throw new Error("No programId provided in instructions");
  if (!req.body.instructions[0].data)
    throw new Error("No data provided in instructions");

  // TODO: add other validation checks and extraction of data as necessary
}

export async function parseReqParams(reqInstructions: any) {
  const instructions: TransactionInstruction[] = [];
  let accounts: AccountMeta[] = [];
  const relayer = getRelayer();
  for (const instruction of reqInstructions) {
    accounts = instruction.keys.map((key: AccountMeta) => {
      return {
        pubkey: new PublicKey(key.pubkey),
        isWritable: key.isWritable,
        isSigner: key.isSigner,
      };
    });
    // checking that relayer is signer and writable
    if (
      accounts[0].pubkey.toBase58() !==
        relayer.accounts.relayerPubkey.toBase58() ||
      accounts[0].isSigner != true ||
      accounts[0].isWritable != true
    )
      throw new Error(
        `Relayer pubkey in instruction != relayer pubkey ${accounts[0].pubkey.toBase58()} ${relayer.accounts.relayerPubkey.toBase58()} not signer ${
          accounts[0].isSigner
        } not writable ${accounts[0].isWritable}}`,
      );
    console.log(
      "account 0",
      accounts[0].pubkey.toBase58() ===
        relayer.accounts.relayerPubkey.toBase58(),
    );
    const newInstruction = new TransactionInstruction({
      keys: accounts,
      programId: new PublicKey(instruction.programId),
      data: Buffer.from(instruction.data),
    });
    instructions.push(newInstruction);
  }
  const relayerRecipientSol = accounts[5];
  console.log(
    "relayerRecipientSol",
    relayerRecipientSol.pubkey.toBase58() ===
      relayer.accounts.relayerRecipientSol.toBase58(),
  );

  // checking that recipient sol is correct
  if (
    relayerRecipientSol.pubkey.toBase58() !==
    relayer.accounts.relayerRecipientSol.toBase58()
  )
    throw new Error(
      `Relayer recipient sol pubkey in instruction != relayer recipient sol pubkey ${relayerRecipientSol.pubkey.toBase58()} ${relayer.accounts.relayerRecipientSol.toBase58()} not signer ${
        relayerRecipientSol.isSigner
      } not writable ${relayerRecipientSol.isWritable}}`,
    );
  return instructions;
}

/**
 * Performs sanity payload checks and adds relay job to queue. Returns signatures and blockhashInfo.
 */
export async function handleRelayRequest(req: any, res: any) {
  try {
    validateReqParams(req);
    // TODO: recognize and handle optional priorityFees field in payload
    const instructions = await parseReqParams(req.body.instructions);
    console.log(
      `/handleRelayRequest - req.body.instructions: ${req.body.instructions}`,
    );

    const { job, signatures, blockhashInfo } = await addRelayJob({
      instructions,
    });

    console.log(
      `/handleRelayRequest - added relay job to queue - id: ${job.id}`,
    );

    return res.status(200).json({
      signatures,
      blockhashInfo,
    } as SignaturesWithBlockhashInfo);
  } catch (error) {
    console.log("/handleRelayRequest error (500)", error, error.message);
    return res.status(500).json({ message: error.message });
  }
}
