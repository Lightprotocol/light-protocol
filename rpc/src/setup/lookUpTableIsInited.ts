import { AnchorProvider } from "@coral-xyz/anchor";
import { RPC_LOOK_UP_TABLE } from "../config";

export const lookUpTableIsInited = async (anchorProvider: AnchorProvider) => {
  const lookUpTableInfoInit =
    await anchorProvider.connection.getAccountInfo(RPC_LOOK_UP_TABLE);
  if (lookUpTableInfoInit) {
    return true;
  }
  return false;
};
