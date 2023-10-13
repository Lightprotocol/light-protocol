import { AnchorProvider } from "@coral-xyz/anchor";
import { RELAYER_LOOK_UP_TABLE } from "../config";

export const lookUpTableIsInited = async (anchorProvider: AnchorProvider) => {
  const lookUpTableInfoInit = await anchorProvider.connection.getAccountInfo(
    RELAYER_LOOK_UP_TABLE,
  );
  if (lookUpTableInfoInit) {
    return true;
  }
  return false;
};
