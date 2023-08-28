import { BN } from "@coral-xyz/anchor";
import { airdropSol } from "@lightprotocol/zk.js";
import { NETWORK, Network } from "../config";
import {
  getAnchorProvider,
  getKeyPairFromEnv,
  getRelayer,
} from "../utils/provider";

export async function fundRelayer() {
  const anchorProvider = await getAnchorProvider();

  const airdropAmount =
    NETWORK === Network.TESTNET
      ? 1000 * 1e6
      : NETWORK === Network.LOCALNET
      ? 1000 * 1e9
      : 1000 * 1e9; // TODO: supply env to CI env, set to 0

  await airdropSol({
    connection: anchorProvider.connection,
    lamports: airdropAmount,
    recipientPublicKey: getKeyPairFromEnv("KEY_PAIR").publicKey,
  });
  const relayer = await getRelayer();
  relayer.relayerFee = new BN(10_000);
  console.log(
    "Relayer Feepayer funded:",
    relayer.accounts.relayerPubkey.toBase58(),
  );
  await airdropSol({
    connection: anchorProvider.connection,
    lamports: airdropAmount,
    recipientPublicKey: relayer.accounts.relayerRecipientSol,
  });
  console.log(
    "Relayer Recipient (SOL) funded:",
    relayer.accounts.relayerRecipientSol.toBase58(),
  );
}
