import { RELAYER_FEE, airdropSol } from "@lightprotocol/zk.js";
import { NETWORK, Network } from "../config";
import {
  getAnchorProvider,
  getKeyPairFromEnv,
  getRelayer,
} from "../utils/provider";

export async function fundRelayer() {
  const anchorProvider = await getAnchorProvider();

  const keyPairPublicKey = getKeyPairFromEnv("KEY_PAIR").publicKey;
  const relayer = await getRelayer();
  const relayerPublicKey = relayer.accounts.relayerRecipientSol;
  relayer.relayerFee = RELAYER_FEE;

  const keyPairBalance = await anchorProvider.connection.getBalance(
    keyPairPublicKey,
  );
  const relayerBalance = await anchorProvider.connection.getBalance(
    relayerPublicKey,
  );

  const airdropAmount =
    NETWORK === Network.TESTNET
      ? 1000 * 1e6
      : NETWORK === Network.LOCALNET
      ? 1000 * 1e9
      : 1000 * 1e9; // TODO: supply env to CI env, set to 0

  if (keyPairBalance > airdropAmount && relayerBalance > airdropAmount) {
    console.log("Relayer keys already funded. Skipping airdrops.");
    return;
  }

  await airdropSol({
    connection: anchorProvider.connection,
    lamports: airdropAmount,
    recipientPublicKey: getKeyPairFromEnv("KEY_PAIR").publicKey,
  });
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
