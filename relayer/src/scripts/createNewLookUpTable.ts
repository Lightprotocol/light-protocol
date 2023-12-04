import { useWallet, initLookUpTable } from "@lightprotocol/zk.js";
import { AddressLookupTableAccount } from "@solana/web3.js";
import { getAnchorProvider, getKeyPairFromEnv } from "utils/provider";

/**
 * Initializes a new LOOK_UP_TABLE account.
 * Run this on public networks if you're setting your own relayer for the first time.
 * Ensure that you're setting the LOOK_UP_TABLE env var to the value printed by this script.
 * FYI: Running relayer tests will override your env var with a fixed base58 value.
 */
(async () => {
  console.log("initing new lookUpTable...");
  const anchorProvider = await getAnchorProvider();
  try {
    const rpc = anchorProvider.connection.rpcEndpoint;
    const wallet = useWallet(getKeyPairFromEnv("KEY_PAIR"), rpc);

    console.log("network: ", rpc);
    const lookUpTable = await initLookUpTable(wallet, anchorProvider);

    const lookUpTableInfoInit =
      await anchorProvider.connection.getAccountInfo(lookUpTable);
    AddressLookupTableAccount.deserialize(lookUpTableInfoInit!.data);

    console.log(
      `Initialized LOOK_UP_TABLE ${lookUpTable.toString()}. \nSet LOOK_UP_TABLE env var to this value.`,
    );
  } catch (e) {
    console.log("error:", e);
    throw new Error(e);
  }
})();
