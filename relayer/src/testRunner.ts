import { ADMIN_AUTH_KEYPAIR, getRecentTransactions, Provider } from "light-sdk";

import { setAnchorProvider } from "./utils/provider";


(async () => {
  await setAnchorProvider();

  const provider = await Provider.init({
    wallet: ADMIN_AUTH_KEYPAIR,
  }); // userKeypair

  const recentTransactions = await getRecentTransactions({
    connection: provider.provider!.connection,
    limit: 1000,
    dedupe: false,
  });

  console.log({ recentTransactions });
})();
