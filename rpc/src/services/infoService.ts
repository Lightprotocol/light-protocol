import { getRpc } from "../utils/provider";

export async function getRpcInfo(_req: any, res: any): Promise<string> {
  try {
    const rpc = getRpc();
    return res.status(200).json({
      data: {
        rpcPubkey: rpc.accounts.rpcPubkey.toBase58(),
        rpcRecipientSol: rpc.accounts.rpcRecipientSol.toBase58(),
        rpcFee: rpc.rpcFee.toString(),
        highRpcFee: rpc.highRpcFee.toString(),
      },
    });
  } catch (e) {
    console.log("@getRpcInfo error: ", e);
    return res.status(500).json({ status: "error", message: e.message });
  }
}
