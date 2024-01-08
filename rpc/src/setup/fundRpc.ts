import { RPC_FEE, airdropSol } from "@lightprotocol/zk.js";
import { NETWORK, Network } from "../config";
import {
  getAnchorProvider,
  getKeyPairFromEnv,
  getRpc,
} from "../utils/provider";
import { PublicKey } from "@solana/web3.js";

export async function fundRpc() {
  const anchorProvider = await getAnchorProvider();

  const rpc = getRpc();
  const rpcPubkey = rpc.accounts.rpcPubkey;
  const rpcRecipient = rpc.accounts.rpcRecipientSol;
  rpc.rpcFee = RPC_FEE;

  const keyPairBalance =
    await anchorProvider.connection.getBalance(rpcPubkey);
  const rpcBalance =
    await anchorProvider.connection.getBalance(rpcRecipient);

  // print balances
  console.log(
    "Rpc Feepayer balance:",
    keyPairBalance,
    rpcPubkey.toBase58(),
  );
  console.log(
    "Rpc Recipient (SOL) balance:",
    rpcBalance,
    rpcRecipient.toBase58(),
  );

  const airdropAmount =
    NETWORK === Network.TESTNET
      ? 1000 * 1e6
      : NETWORK === Network.DEVNET
      ? 1000 * 1e6
      : NETWORK === Network.LOCALNET
      ? 1000 * 1e9
      : 1000 * 1e9; // TODO: supply env to CI env, set to 0

  const fundAccount = async (
    balance: number,
    account: PublicKey,
    accountName: string,
  ) => {
    if (balance > airdropAmount) {
      console.log(`${accountName} key already funded. Skipping airdrop.`);
    } else {
      try {
        await airdropSol({
          connection: anchorProvider.connection,
          lamports: airdropAmount,
          recipientPublicKey: account,
        });
        console.log(
          `${accountName} funded:`,
          (
            rpc.accounts[
              accountName as keyof typeof rpc.accounts
            ] as PublicKey
          ).toBase58(),
        );
      } catch (e) {
        throw new Error(`Error funding ${accountName} ${e}`);
      }
    }
  };

  await fundAccount(
    keyPairBalance,
    getKeyPairFromEnv("KEY_PAIR").publicKey,
    "rpcPubkey",
  );
  await fundAccount(
    rpcBalance,
    rpc.accounts.rpcRecipientSol,
    "rpcRecipientSol",
  );
}
