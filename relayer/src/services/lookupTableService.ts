import { readLookupTable } from "../utils/readLookupTable";
import { getAnchorProvider } from "../utils/provider";

export async function getLookUpTable(_req: any, res: any): Promise<string> {
  try {
    const contents = readLookupTable();
    const provider = await getAnchorProvider();
    const info = await provider.connection.getAccountInfo(contents);
    console.log("@getLookUpTable accInfo:", info, "pub:", contents);
    if (!info) throw new Error("accInfo is null");
    return res.status(200).json({ data: contents });
  } catch (e) {
    console.log("@getLookUpTable error: ", e);
    return res.status(500).json({ status: "error", message: e.message });
  }
}
