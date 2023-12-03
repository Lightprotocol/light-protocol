import { RELAYER_FEE, airdropSol } from "@lightprotocol/zk.js";
import { NETWORK, Network } from "../config";
import {
  getAnchorProvider,
  getKeyPairFromEnv,
  getRelayer,
} from "../utils/provider";
import { PublicKey } from "@solana/web3.js";

export async function fundRelayer() {
  const anchorProvider = await getAnchorProvider();

  const relayer = getRelayer();
  const relayerPubkey = relayer.accounts.relayerPubkey;
  const relayerRecipient = relayer.accounts.relayerRecipientSol;
  relayer.relayerFee = RELAYER_FEE;

  const keyPairBalance =
    await anchorProvider.connection.getBalance(relayerPubkey);
  const relayerBalance =
    await anchorProvider.connection.getBalance(relayerRecipient);

  // print balances
  console.log(
    "Relayer Feepayer balance:",
    keyPairBalance,
    relayerPubkey.toBase58(),
  );
  console.log(
    "Relayer Recipient (SOL) balance:",
    relayerBalance,
    relayerRecipient.toBase58(),
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
            relayer.accounts[
              accountName as keyof typeof relayer.accounts
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
    "relayerPubkey",
  );
  await fundAccount(
    relayerBalance,
    relayer.accounts.relayerRecipientSol,
    "relayerRecipientSol",
  );
}
