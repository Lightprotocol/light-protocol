import * as anchor from "@coral-xyz/anchor";
import {
  Keypair,
  Keypair as SolanaKeypair,
  SystemProgram,
} from "@solana/web3.js";
import { Idl } from "@coral-xyz/anchor";
const token = require("@solana/spl-token");

// TODO: add and use namespaces in SDK
import {
  Transaction,
  Utxo,
  LOOK_UP_TABLE,
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MINT,
  Provider,
  AUTHORITY_ONE,
  createTestAccounts,
  userTokenAccount,
  recipientTokenAccount,
  FEE_ASSET,
  confirmConfig,
  TransactionParameters,
  User,
  Action,
  TestRelayer,
  TestTransaction,
  IDL_LIGHT_PSP2IN2OUT,
  IDL_LIGHT_PSP10IN2OUT,
  IDL_LIGHT_PSP2IN2OUT_STORAGE,
  Account,
  airdropSol,
  MerkleTreeConfig,
  RELAYER_FEE,
  BN_0,
  airdropSplToAssociatedTokenAccount,
  SolanaTransactionInputs,
  getSystemProof,
  createSystemProofInputs,
  createSolanaInstructions,
  getSolanaRemainingAccounts,
} from "../../src";
import { WasmHasher, Hasher } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";

let HASHER: Hasher;
let RELAYER: TestRelayer;
let ACCOUNT: Account;

// TODO: remove deprecated function calls
describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  anchor.setProvider(provider);

  before("init test setup Merkle tree lookup table etc", async () => {
    await createTestAccounts(provider.connection, userTokenAccount);

    HASHER = await WasmHasher.getInstance();
    const seed = bs58.encode(new Uint8Array(32).fill(1));
    ACCOUNT = new Account({
      hasher: HASHER,
      seed,
    });

    const relayerRecipientSol = SolanaKeypair.generate().publicKey;

    await provider.connection.requestAirdrop(relayerRecipientSol, 2e9);

    RELAYER = new TestRelayer({
      relayerPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      relayerRecipientSol,
      relayerFee: RELAYER_FEE,
      payer: ADMIN_AUTH_KEYPAIR,
    });
  });

  it("Shield (verifier one)", async () => {
    await performShield({
      delegate: AUTHORITY_ONE,
      spl: true,
      shuffleEnabled: true,
      verifierIdl: IDL_LIGHT_PSP10IN2OUT,
    });
  });

  it("Shield (verifier storage)", async () => {
    await performShield({
      delegate: AUTHORITY,
      spl: false,
      message: Buffer.alloc(900).fill(1),
      shuffleEnabled: false,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT_STORAGE,
    });
  });

  it("Shield (verifier zero)", async () => {
    await performShield({
      delegate: AUTHORITY,
      spl: true,
      shuffleEnabled: true,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT,
    });
  });

  it("Unshield (verifier zero)", async () => {
    await performUnshield({
      outputUtxos: [],
      tokenProgram: MINT,
      recipientSpl: recipientTokenAccount,
      shuffleEnabled: false,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT,
    });
  });

  it("Unshield (verifier storage)", async () => {
    await performUnshield({
      outputUtxos: [],
      tokenProgram: SystemProgram.programId,
      message: Buffer.alloc(900).fill(1),
      shuffleEnabled: false,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT_STORAGE,
    });
  });

  it("Unshield (verifier one)", async () => {
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });
    const user: User = await User.init({
      provider: lightProvider,
      account: ACCOUNT,
    });
    const inputUtxos: Utxo[] = [
      user.balance.tokenBalances.get(MINT.toBase58())!.utxos.values().next()
        .value,
    ];
    await performUnshield({
      outputUtxos: [
        new Utxo({
          hasher: HASHER,
          publicKey: inputUtxos[0].publicKey,
          assets: inputUtxos[0].assets,
          amounts: [BN_0, inputUtxos[0].amounts[1]],
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        }),
      ],
      tokenProgram: MINT,
      recipientSpl: recipientTokenAccount,
      shuffleEnabled: true,
      verifierIdl: IDL_LIGHT_PSP10IN2OUT,
    });
  });

  const performShield = async ({
    delegate,
    spl = false,
    message,
    shuffleEnabled = true,
    verifierIdl,
  }: {
    delegate: anchor.web3.PublicKey;
    spl: boolean;
    message?: Buffer;
    shuffleEnabled: boolean;
    verifierIdl: Idl;
  }) => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }

    const shieldAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
    const shieldFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

    await airdropSplToAssociatedTokenAccount(
      provider.connection,
      10e9,
      ADMIN_AUTH_KEYPAIR.publicKey,
    );
    const tokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      ADMIN_AUTH_KEYPAIR,
      MINT,
      ADMIN_AUTH_KEYPAIR.publicKey,
    );
    await token.approve(
      provider.connection,
      ADMIN_AUTH_KEYPAIR,
      tokenAccount.address,
      delegate, // delegate
      ADMIN_AUTH_KEYPAIR.publicKey, // owner
      shieldAmount * 2,
      [ADMIN_AUTH_KEYPAIR],
    );
    const senderSpl = tokenAccount.address;
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });

    const shieldUtxo1 = spl
      ? new Utxo({
          hasher: HASHER,
          assets: [FEE_ASSET, MINT],
          amounts: [
            new anchor.BN(shieldFeeAmount),
            new anchor.BN(shieldAmount),
          ],
          publicKey: ACCOUNT.pubkey,
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        })
      : new Utxo({
          hasher: HASHER,
          amounts: [new anchor.BN(shieldFeeAmount)],
          publicKey: ACCOUNT.pubkey,
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
        });

    const txParams = new TransactionParameters({
      outputUtxos: [shieldUtxo1],
      message,
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      senderSpl,
      senderSol: ADMIN_AUTH_KEYPAIR.publicKey,
      action: Action.SHIELD,
      hasher: HASHER,
      verifierIdl: verifierIdl,
      account: ACCOUNT,
    });
    await txParams.getTxIntegrityHash(HASHER);
    const transactionTester = new TestTransaction({
      txParams,
      provider: lightProvider,
    });
    await transactionTester.getTestValues();

    const systemProofInputs = createSystemProofInputs({
      transaction: txParams,
      solMerkleTree: lightProvider.solMerkleTree!,
      hasher: HASHER,
      account: ACCOUNT,
    });
    const systemProof = await getSystemProof({
      account: ACCOUNT,
      transaction: txParams,
      systemProofInputs,
    });
    const { rootIndex, remainingAccounts: remainingMerkleTreeAccounts } =
      await lightProvider.getRootIndex();

    const remainingSolanaAccounts = getSolanaRemainingAccounts(
      systemProof.parsedPublicInputsObject,
      remainingMerkleTreeAccounts,
    );

    // createSolanaInstructionsWithAccounts
    const instructions = await createSolanaInstructions(
      rootIndex,
      systemProof,
      remainingSolanaAccounts,
      txParams,
      verifierIdl,
    );
    await lightProvider.sendAndConfirmShieldedTransaction(instructions);

    await transactionTester.checkBalances(
      { publicInputs: systemProof.parsedPublicInputsObject },
      remainingSolanaAccounts,
      systemProofInputs,
      ACCOUNT,
    );
  };

  const performUnshield = async ({
    outputUtxos,
    tokenProgram,
    message,
    recipientSpl,
    shuffleEnabled = true,
    verifierIdl,
  }: {
    outputUtxos: Array<Utxo>;
    tokenProgram: anchor.web3.PublicKey;
    message?: Buffer;
    recipientSpl?: anchor.web3.PublicKey;
    shuffleEnabled: boolean;
    verifierIdl: Idl;
  }) => {
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });
    const user = await User.init({
      provider: lightProvider,
      account: ACCOUNT,
    });

    const origin = Keypair.generate();
    await airdropSol({
      connection: lightProvider.provider.connection,
      lamports: 1000 * 1e9,
      recipientPublicKey: origin.publicKey,
    });

    const txParams = new TransactionParameters({
      inputUtxos: [
        user.balance.tokenBalances
          .get(tokenProgram.toBase58())!
          .utxos.values()
          .next().value,
      ],
      outputUtxos,
      message,
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      recipientSpl,
      recipientSol: origin.publicKey,
      relayer: RELAYER,
      action: Action.UNSHIELD,
      hasher: HASHER,
      verifierIdl: verifierIdl,
      account: ACCOUNT,
    });
    await txParams.getTxIntegrityHash(HASHER);
    const transactionTester = new TestTransaction({
      txParams,
      provider: lightProvider,
    });
    await transactionTester.getTestValues();

    const systemProofInputs = createSystemProofInputs({
      transaction: txParams,
      solMerkleTree: lightProvider.solMerkleTree!,
      hasher: HASHER,
      account: ACCOUNT,
    });
    const systemProof = await getSystemProof({
      account: ACCOUNT,
      transaction: txParams,
      systemProofInputs,
    });
    const { rootIndex, remainingAccounts: remainingMerkleTreeAccounts } =
      await lightProvider.getRootIndex();

    const remainingSolanaAccounts = getSolanaRemainingAccounts(
      systemProof.parsedPublicInputsObject,
      remainingMerkleTreeAccounts,
    );

    // createSolanaInstructionsWithAccounts
    const instructions = await createSolanaInstructions(
      rootIndex,
      systemProof,
      remainingSolanaAccounts,
      txParams,
      verifierIdl,
    );
    await lightProvider.sendAndConfirmShieldedTransaction(instructions);

    await transactionTester.checkBalances(
      { publicInputs: systemProof.parsedPublicInputsObject },
      remainingSolanaAccounts,
      systemProofInputs,
      ACCOUNT,
    );
  };
});
