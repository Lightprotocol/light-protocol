import { PublicKey } from "@solana/web3.js";
import {
  ADMIN_AUTH_KEYPAIR,
  getRecentTransactions,
  Provider,
  TestRelayer,
} from "light-sdk";
import { BN } from "@coral-xyz/anchor";
import { setAnchorProvider } from "./utils/provider";

(async () => {
  await setAnchorProvider();

  const provider = await Provider.init({
    wallet: ADMIN_AUTH_KEYPAIR,
  }); // userKeypair

  const relayer = await new TestRelayer(
    ADMIN_AUTH_KEYPAIR.publicKey,
    new PublicKey("FbzCWM3EfMU6YhVukNZo86DuL9WLoDrMStztqkgAodKf"),
    ADMIN_AUTH_KEYPAIR.publicKey,
    new BN(100000),
  );

  const transaction = await relayer.getTransactionHistory(provider.provider!.connection);
  console.log({transaction})
})();
