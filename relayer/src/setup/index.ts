import { initLookUpTable, useWallet } from "@lightprotocol/zk.js";
import { getKeyPairFromEnv } from "../utils/provider";
import { AddressLookupTableAccount, PublicKey } from "@solana/web3.js";
import { RPC_URL } from "../config";
import { AnchorProvider } from "@coral-xyz/anchor";

export async function setupRelayerLookUpTable(anchorProvider: AnchorProvider) {
  let lookUpTable;

  try {
    let lookUpTableRead = new PublicKey(process.env.LOOK_UP_TABLE!);
    let lookUpTableInfoInit = await anchorProvider.connection.getAccountInfo(
      lookUpTableRead,
    );
    if (lookUpTableInfoInit) {
      lookUpTable = lookUpTableRead;
    }
    AddressLookupTableAccount.deserialize(lookUpTableInfoInit!.data);
  } catch (e) {
    console.log(".look_up_table env not found or not properly initialized", e);
  }
  if (!lookUpTable) {
    console.log("initing lookuptable...");
    let wallet = useWallet(getKeyPairFromEnv("KEY_PAIR"), RPC_URL);

    lookUpTable = await initLookUpTable(wallet, anchorProvider);
    console.log("new relayer lookUpTable created: ", lookUpTable.toString());
    process.env.LOOK_UP_TABLE = lookUpTable.toString();
    console.log(".env updated with:", process.env.LOOK_UP_TABLE);
  }
}
