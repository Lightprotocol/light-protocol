import { getRelayer } from "../utils/provider";

export async function getRelayerInfo(_req: any, res: any): Promise<string> {
  try {
    const relayer = await getRelayer();
    return res.status(200).json({
      data: {
        relayerPubkey: relayer.accounts.relayerPubkey.toBase58(),
        relayerRecipientSol: relayer.accounts.relayerRecipientSol.toBase58(),
        relayerFee: relayer.relayerFee.toString(),
        highRelayerFee: relayer.highRelayerFee.toString(),
      },
    });
  } catch (e) {
    console.log("@getRelayerInfo error: ", e);
    return res.status(500).json({ status: "error", message: e.message });
  }
}
