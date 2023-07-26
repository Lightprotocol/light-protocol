import { initLookUpTableFromFile } from "@lightprotocol/zk.js";
import { getLightProvider } from "../utils/provider";

export const initLookupTable = async (req: any, res: any) => {
  try {
    const provider = await getLightProvider();
    const lookupTable = await initLookUpTableFromFile(provider.provider!);
    return res.status(200).json({ data: lookupTable });
  } catch (e) {
    return res.status(500).json({ status: "error", message: e.message });
  }
};
