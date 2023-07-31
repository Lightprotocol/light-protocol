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
    response: null,
  });
  return job;
}
