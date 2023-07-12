import { createTestAccounts } from "@lightprotocol/zk.js";
import { setAnchorProvider } from "../utils/provider";

export const testSetup = async () => {
  const providerAnchor = await setAnchorProvider();
  // TODO: use updated -- buildscript -> add relayer tests
  await createTestAccounts(providerAnchor.connection);
};
