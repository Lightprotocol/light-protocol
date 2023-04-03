import {
  ADMIN_AUTH_KEYPAIR,
  initLookUpTableFromFile,
  Provider,
} from "light-sdk";
import { getLightProvider } from "utils/provider";

export const initLookupTable = async (req: any, res: any) => {
  try {
    const provider = await getLightProvider();
    const LOOK_UP_TABLE = await initLookUpTableFromFile(provider.provider!);
    return res.status(200).json({ data: LOOK_UP_TABLE });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error" });
  }
};
