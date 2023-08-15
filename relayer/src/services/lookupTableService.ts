import { readLookupTable } from "../utils/readLookupTable";
import { getAnchorProvider } from "../utils/provider";

import { PublicKey } from "@solana/web3.js";

export async function getLookUpTable(_req: any, res: any): Promise<string> {
  try {
    let contents = readLookupTable();
    let provider = await getAnchorProvider();
    let info = await provider.connection.getAccountInfo(
      new PublicKey(contents),
    );
    console.log("@getLookUpTable accInfo:", info, "pub:", contents);
    if (!info) throw new Error("accInfo is null");
    return res.status(200).json({ data: contents });
  } catch (e) {
    console.log("@getLookUpTable error: ", e);
    return res.status(500).json({ status: "error", message: e.message });
  }
}
