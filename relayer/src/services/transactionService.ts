import { TransactionSignature } from "@solana/web3.js";
import {
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";
import { sendVersionedTransaction } from "light-sdk";
import { getLightProvider } from "../utils/provider";

export async function sendTransaction(req: any, res: any) {
  try {
    if (!req.body.instruction) throw new Error("No instructions provided");
    const provider = await getLightProvider();
    if (!provider.provider) throw new Error("no provider set");

    const instruction = req.body.instruction;

    const accounts = instruction.keys.map((key: any) => {
      return {
        pubkey: new PublicKey(key.pubkey),
        isWritable: key.isWritable,
        isSigner: key.isSigner,
      };
    });

    const newInstruction = new TransactionInstruction({
      keys: accounts,
      programId: new PublicKey(instruction.programId),
      data: Buffer.from(instruction.data),
    });
    const response = await sendVersionedTransaction(newInstruction, provider);
    return res.status(200).json({ data: response });
  } catch (error) {
    return res.status(500).json({ status: "error", message: error.message });
  }
}
