import { TransactionSignature } from "@solana/web3.js";
import {
  ADMIN_AUTH_KEYPAIR,
  Provider,
  sendVersionedTransaction,
} from "light-sdk";
import { getLightProvider } from "utils/provider";

export async function sendTransaction(
  req: any,
  res: any,
): Promise<TransactionSignature | undefined> {
  try {
    if (!req.body.instructions) throw new Error("No instructions provided");
    const provider = await getLightProvider();
    if (!provider.provider) throw new Error("no provider set");
    const response = sendVersionedTransaction(req.body.instructions, provider);
    return res.status("").json({ data: response });
  } catch (error) {
    console.log({ error });
    return res.status(500).json({ status: "error" });
  }
}
