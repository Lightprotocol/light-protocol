import * as anchor from "@coral-xyz/anchor";
import {
  Transaction,
  Provider as LightProvider,
  confirmConfig,
  TestRelayer,
  User,
  airdropSol,
  verifierProgramStorageProgramId,
  merkleTreeProgramId,
} from "@lightprotocol/zk.js";
import { Keypair } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { MessageClient } from "./helpers";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

const RPC_URL = "http://127.0.0.1:8899";

describe("Test foobar", () => {
  const provider = setupAnchor();

  it("Test encrypted messaging", async () => {
    const authorityPda = Transaction.getSignerAuthorityPda(
      merkleTreeProgramId,
      verifierProgramStorageProgramId
    );
    const authorityBalance =
      (await provider.connection.getBalance(authorityPda)) / 1e9;
    console.log(`authorityPda balance: ${authorityBalance} SOL`);

    const wallet = await createWalletAndAirdropSol(provider, 1e10);
    const relayer = createRelayer(wallet);

    let lightProvider = await LightProvider.init({
      wallet,
      url: RPC_URL,
      relayer,
      confirmConfig: confirmConfig,
    });
    const user: User = await User.init({ provider: lightProvider });

    let messageClient = new MessageClient(user);

    let seed = new Uint8Array(32).fill(1);
    let encodedSeed = bs58.encode(seed);
    let recipient = (await User.init({
      provider: lightProvider,
      seed: encodedSeed,
    })) as User;

    await messageClient.encryptAndStoreForRecipient(
      "foobaz",
      recipient.account.encryptionKeypair.publicKey
    );

    const recipientMessageClient = new MessageClient(recipient);
    await recipientMessageClient.getMessages();
  });
});

function setupAnchor(): anchor.AnchorProvider {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);
  return provider;
}

async function createWalletAndAirdropSol(
  provider: anchor.AnchorProvider,
  amount: number
): Promise<Keypair> {
  const wallet = Keypair.generate();
  await airdropSol({
    connection: provider.connection,
    lamports: amount,
    recipientPublicKey: wallet.publicKey,
  });
  return wallet;
}

function createRelayer(wallet: Keypair): TestRelayer {
  return new TestRelayer({
    relayerPubkey: wallet.publicKey,
    relayerRecipientSol: wallet.publicKey,
    relayerFee: new BN(100_000),
    payer: wallet,
  });
}
