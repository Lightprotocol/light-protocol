import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, Program } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  IDL_USER_REGISTRY,
  userRegistryProgramId,
  confirmConfig,
  UserRegistry,
  ADMIN_AUTH_KEYPAIR,
  ADMIN_AUTH_KEY,
  createTestAccounts,
  userTokenAccount,
  Account,
  Provider,
  TestRelayer,
  RELAYER_FEE,
} from "@lightprotocol/zk.js";
import {
  Keypair as SolanaKeypair,
  PublicKey,
  sendAndConfirmTransaction,
  SystemProgram,
} from "@solana/web3.js";
import { assert } from "chai";
const circomlibjs = require("circomlibjs");

let KEYPAIR: Account, RELAYER: TestRelayer;

describe("User registry", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const provider = AnchorProvider.local("http://127.0.0.1:8899", confirmConfig);
  anchor.setProvider(provider);

  const userRegistryProgram: Program<UserRegistry> = new Program(
    IDL_USER_REGISTRY,
    userRegistryProgramId,
    provider,
  );

  before("Create user", async () => {
    await createTestAccounts(provider.connection, userTokenAccount);

    let poseidon = await circomlibjs.buildPoseidonOpt();
    const seed = bs58.encode(new Uint8Array(32).fill(1));
    KEYPAIR = new Account({
      poseidon: poseidon,
      seed,
    });

    const relayerRecipientSol = SolanaKeypair.generate().publicKey;

    await provider.connection.requestAirdrop(
      relayerRecipientSol,
      2_000_000_000,
    );

    RELAYER = new TestRelayer({
      relayerPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      relayerRecipientSol,
      relayerFee: RELAYER_FEE,
      payer: ADMIN_AUTH_KEYPAIR,
    });

    await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });
  });

  it("Register user", async () => {
    const userEntryPubkey = PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("user-entry"), ADMIN_AUTH_KEY.toBuffer()],
      userRegistryProgramId,
    )[0];
    const tx = await userRegistryProgram.methods
      .initializeUserEntry(KEYPAIR.pubkey.toArray(), [
        ...KEYPAIR.encryptionKeypair.publicKey,
      ])
      .accounts({
        signer: ADMIN_AUTH_KEYPAIR.publicKey,
        systemProgram: SystemProgram.programId,
        userEntry: userEntryPubkey,
      })
      .signers([ADMIN_AUTH_KEYPAIR])
      .transaction();
    await sendAndConfirmTransaction(
      provider.connection,
      tx,
      [ADMIN_AUTH_KEYPAIR],
      confirmConfig,
    );

    let accountInfo = await userRegistryProgram.account.userEntry.fetch(
      userEntryPubkey,
    );
    assert.deepEqual(accountInfo.lightPubkey, KEYPAIR.pubkey.toArray());
    assert.deepEqual(accountInfo.lightEncryptionPubkey, [
      ...KEYPAIR.encryptionKeypair.publicKey,
    ]);
    assert.deepEqual(
      new Uint8Array(accountInfo.solanaPubkey),
      ADMIN_AUTH_KEY.toBytes(),
    );
  });
});
