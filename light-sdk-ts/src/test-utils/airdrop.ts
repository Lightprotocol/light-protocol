import { AnchorProvider } from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { Provider, User } from "../index";

export async function airdropShieldedSol({
  provider,
  amount,
  seed,
  recipientPublicKey,
}: {
  provider: Provider;
  amount: number;
  seed?: string;
  recipientPublicKey?: string;
}) {
  if (!amount) throw new Error("Sol Airdrop amount undefined");
  if (!seed && !recipientPublicKey)
    throw new Error(
      "Sol Airdrop seed and recipientPublicKey undefined define a seed to airdrop shielded sol aes encrypted, define a recipientPublicKey to airdrop shielded sol to the recipient nacl box encrypted",
    );

  const userKeypair = Keypair.generate();
  let res = await provider.provider!.connection.requestAirdrop(
    userKeypair.publicKey,
    amount,
  );
  await provider.provider!.connection.confirmTransaction(res, "confirmed");
  const user: User = await User.init({ provider, seed });
  return await user.shield({
    publicAmountSol: amount,
    token: "SOL",
  });
}

export async function airdropSol({
  provider,
  amount,
  recipientPublicKey,
}: {
  provider: AnchorProvider;
  amount: number;
  recipientPublicKey: PublicKey;
}) {
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(recipientPublicKey, amount),
    "confirmed",
  );
}
