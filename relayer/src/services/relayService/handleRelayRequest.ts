import {
  AccountMeta,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { sleep } from "@lightprotocol/zk.js";
import { relayQueue } from "../../db/redis";
import { Job } from "bullmq";
import { MAX_STEPS_TO_WAIT_FOR_JOB_COMPLETION, SECONDS } from "../../config";
import { getRelayer } from "../../utils/provider";

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
  let instructions: TransactionInstruction[] = [];
  let accounts: AccountMeta[] = [];
  let relayer = await getRelayer();
  for (let instruction of reqInstructions) {
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
    let newInstruction = new TransactionInstruction({
      keys: accounts,
      programId: new PublicKey(instruction.programId),
      data: Buffer.from(instruction.data),
    });
    instructions.push(newInstruction);
  }
  console.log(
    "account 10",
    accounts[10].pubkey.toBase58() ===
      relayer.accounts.relayerRecipientSol.toBase58(),
  );

  // checking that recipient sol is correct
  if (
    accounts[10].pubkey.toBase58() !==
    relayer.accounts.relayerRecipientSol.toBase58()
  )
    // || accounts[10].isSigner != false || accounts[10].isWritable != true
    throw new Error(
      `Relayer recipient sol pubkey in instruction != relayer recipient sol pubkey ${accounts[10].pubkey.toBase58()} ${relayer.accounts.relayerRecipientSol.toBase58()} not signer ${
        accounts[10].isSigner
      } not writable ${accounts[10].isWritable}}`,
    );
  return instructions;
}

async function awaitJobCompletion({ job, res }: { job: Job; res: any }) {
  console.log(`/awaitJobCompletion - id: ${job.id}`);
  let state;
  let i = 0;
  let maxSteps = MAX_STEPS_TO_WAIT_FOR_JOB_COMPLETION;
  let sleepTime = 1 * SECONDS;
  while (i < maxSteps) {
    await sleep(sleepTime);
    state = await job.getState();
    if (state === "completed" || state === "failed" || state === "unknown") {
      i = maxSteps;
      if (state === "failed") {
        console.log(`/awaitJobCompletion error (500) failed - id: ${job.id}`);
        return res.status(500).json({ status: "error", message: "500" });
      } else {
        console.log(`/awaitJobCompletion success - id: ${job.id}`);
        return res.status(200).json({
          data: {
            transactionStatus: "confirmed",
            response: job.data.response,
          },
        });
      }
    } else i++;
  }
}

export async function addRelayJob({
  instructions,
}: {
  instructions: TransactionInstruction[];
}) {
  // id is the last pubkey .toBase58() of the last instruction
  // this should be a leaves pda pubkey
  const lastInstruction = instructions[instructions.length - 1];
  let id =
    lastInstruction.keys[lastInstruction.keys.length - 1].pubkey.toBase58();
  let job = await relayQueue.add(id, {
    instructions,
    response: null,
  });
  return job;
}

export async function handleRelayRequest(req: any, res: any) {
  try {
    validateReqParams(req);
    const instructions = await parseReqParams(req.body.instructions);
    console.log(
      `/handleRelayRequest - req.body.instructions: ${req.body.instructions}`,
    );

    const job = await addRelayJob({ instructions });
    console.log(`/handleRelayRequest - added relay job to queue`);

    await awaitJobCompletion({ job, res });
    return;
  } catch (error) {
    console.log("/handleRelayRequest error (500)", error, error.message);
    return res.status(500).json({ status: "error", message: error.message });
  }
}
