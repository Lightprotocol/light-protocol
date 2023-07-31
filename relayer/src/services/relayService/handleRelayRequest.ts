import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { sleep } from "@lightprotocol/zk.js";
import { addRelayJob } from "./addRelayJob";
import { generateNonce } from "../../utils/generateNonce";
import { Job } from "bullmq";
import { MAX_STEPS_TO_WAIT_FOR_JOB_COMPLETION, SECONDS } from "../../config";

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

export function parseReqParams(reqInstructions: any) {
  let instructions: TransactionInstruction[] = [];
  for (let instruction of reqInstructions) {
    let accounts = instruction.keys.map((key: any) => {
      return {
        pubkey: new PublicKey(key.pubkey),
        isWritable: key.isWritable,
        isSigner: key.isSigner,
      };
    });
    let newInstruction = new TransactionInstruction({
      keys: accounts,
      programId: new PublicKey(instruction.programId),
      data: Buffer.from(instruction.data),
    });
    instructions.push(newInstruction);
  }
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

export async function handleRelayRequest(req: any, res: any) {
  try {
    validateReqParams(req);
    const instructions = parseReqParams(req.body.instructions);
    console.log(
      `/handleRelayRequest - req.body.instructions: ${req.body.instructions}`,
    );

    const nonce = generateNonce();
    const job = await addRelayJob({ instructions, nonce });
    console.log(`/handleRelayRequest - added relay job to queue`);

    await awaitJobCompletion({ job, res });
    return;
  } catch (error) {
    console.log("/handleRelayRequest error (500)", error, error.message);
    return res.status(500).json({ status: "error", message: error.message });
  }
}
