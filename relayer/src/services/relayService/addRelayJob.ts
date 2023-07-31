import { Provider } from "@lightprotocol/zk.js";
import { TransactionInstruction } from "@solana/web3.js";
import { relayQueue } from "../../db/redis";

export async function addRelayJob({
  instructions,
  nonce,
}: {
  instructions: TransactionInstruction[];
  nonce: string;
}) {
  let job = await relayQueue.add(nonce, {
    instructions,
    // provider,
    // storing job id in data for better handling of job deletion and other debugging
    // nonce,
    response: null,
  });
  return job;
}
