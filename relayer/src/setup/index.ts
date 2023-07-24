import {
  createTestAccounts,
  initLookUpTableFromFile,
} from "@lightprotocol/zk.js";
import { getAnchorProvider } from "../utils/provider";

export const testSetup = async () => {
  const providerAnchor = await getAnchorProvider();
  // TODO: use updated -- buildscript -> add relayer tests
  await createTestAccounts(providerAnchor.connection);

  await initLookUpTableFromFile(providerAnchor);
};
