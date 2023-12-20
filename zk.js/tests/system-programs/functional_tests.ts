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
  Utxo,
  LOOK_UP_TABLE,
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MINT,
  Provider,
  AUTHORITY_ONE,
  createTestAccounts,
  userTokenAccount,
  FEE_ASSET,
  confirmConfig,
  User,
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
  getSystemProof,
  createSystemProofInputs,
  createSolanaInstructions,
  getSolanaRemainingAccounts,
  ShieldTransactionInput,
  createShieldTransaction,
  prepareAccounts,
  getVerifierProgramId,
  createUnshieldTransaction,
  UnshieldTransactionInput,
  sleep,
} from "../../src";
import { WasmHasher, Hasher } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  getAssociatedTokenAddress,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";

let HASHER: Hasher;
let RELAYER: TestRelayer;
let ACCOUNT: Account;
let lightProvider: Provider;

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
    lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });
    // TODO: apparently lookuptable extends until 33 slot!
    await sleep(15000);
  });

  it.only("Shield (verifier one)", async () => {
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
      spl: true,
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
      spl: false,
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

    console.log("LOOK_UP_TABLE", LOOK_UP_TABLE.toBase58());
    const shieldAmount = spl
      ? 10_000 + Math.floor(Math.random() * 1_000_000_000)
      : 0;
    const shieldFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

    console.log("shieldAmount", shieldAmount);
    console.log("shieldFeeAmount", shieldFeeAmount);

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
    const senderSpl = spl ? tokenAccount.address : undefined;

    // TEST REMOVED lightProvider here

    const shieldUtxo = spl
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

    const shieldTransactionInput: ShieldTransactionInput = {
      hasher: HASHER,
      mint: shieldAmount > 0 ? MINT : undefined,
      message,
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      senderSpl,
      signer: ADMIN_AUTH_KEYPAIR.publicKey,
      systemPspId: getVerifierProgramId(verifierIdl),
      account: ACCOUNT,
      outputUtxos: [shieldUtxo],
      root: lightProvider.solMerkleTree!.merkleTree.root(),
    };

    const shieldTransaction = await createShieldTransaction(
      shieldTransactionInput,
    );

    const systemProofInputs = createSystemProofInputs({
      transaction: shieldTransaction,
      hasher: HASHER,
      account: ACCOUNT,
    });
    const systemProof = await getSystemProof({
      account: ACCOUNT,
      inputUtxos: shieldTransaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs,
    });

    const { rootIndex, remainingAccounts: remainingMerkleTreeAccounts } =
      await lightProvider.getRootIndex();

    const remainingSolanaAccounts = getSolanaRemainingAccounts(
      systemProof.parsedPublicInputsObject as any,
      remainingMerkleTreeAccounts,
    );
    const accounts = prepareAccounts({
      transactionAccounts: shieldTransaction.public.accounts,
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
    });
    // createSolanaInstructionsWithAccounts
    const instructions = await createSolanaInstructions({
      action: shieldTransaction.action,
      rootIndex,
      systemProof,
      remainingSolanaAccounts,
      accounts,
      publicTransactionVariables: shieldTransaction.public,
      systemPspIdl: verifierIdl,
    });

    const transactionTester = new TestTransaction({
      transaction: shieldTransaction,
      accounts,
      provider: lightProvider,
    });
    await transactionTester.getTestValues();
    console.log("@performShield .relayer.sendAndConfirmSolanaInstructions");
    const signatures = await lightProvider.sendAndConfirmSolanaInstructions(
      instructions,
      // lightProvider.connection!,
      { commitment: "confirmed" },
      undefined,
      undefined,
      // lightProvider,
    );
    console.log("SIGS", signatures);

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
    spl,
    shuffleEnabled = true,
    verifierIdl,
  }: {
    outputUtxos: Array<Utxo>;
    tokenProgram: anchor.web3.PublicKey;
    message?: Buffer;
    spl?: boolean;
    shuffleEnabled: boolean;
    verifierIdl: Idl;
  }) => {
    // const lightProvider = await Provider.init({
    //   wallet: ADMIN_AUTH_KEYPAIR,
    //   relayer: RELAYER,
    //   confirmConfig,
    // });
    // FIX: apparently lookuptable extends until 33 slot is finalized!
    // I suspect the same underlying issue wrt my test-validator to cause this
    // Expects finalized state.
    await sleep(15000);

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
    const ata = await getAssociatedTokenAddress(MINT, origin.publicKey);

    const unshieldUtxo = user.balance.tokenBalances
      .get(tokenProgram.toBase58())!
      .utxos.values()
      .next().value;

    // Running into memory issues with verifier one (10in2out) unshielding spl
    const unshieldTransactionInput: UnshieldTransactionInput = {
      hasher: HASHER,
      mint: spl ? MINT : undefined,
      message,
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      recipientSpl: spl ? ata : undefined,
      recipientSol: origin.publicKey,
      relayerPublicKey: lightProvider.relayer.accounts.relayerPubkey,
      systemPspId: getVerifierProgramId(verifierIdl),
      account: ACCOUNT,
      inputUtxos: [unshieldUtxo],
      outputUtxos,
      relayerFee: user.provider.relayer.getRelayerFee(true),
      ataCreationFee: spl ? spl : false,
      root: lightProvider.solMerkleTree!.merkleTree.root(),
    };

    const unshieldTransaction = await createUnshieldTransaction(
      unshieldTransactionInput,
    );

    const systemProofInputs = createSystemProofInputs({
      transaction: unshieldTransaction,
      hasher: HASHER,
      account: ACCOUNT,
    });
    const systemProof = await getSystemProof({
      account: ACCOUNT,
      inputUtxos: unshieldTransaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs,
    });

    const { rootIndex, remainingAccounts: remainingMerkleTreeAccounts } =
      await lightProvider.getRootIndex();

    const remainingSolanaAccounts = getSolanaRemainingAccounts(
      systemProof.parsedPublicInputsObject as any,
      remainingMerkleTreeAccounts,
    );
    const accounts = prepareAccounts({
      transactionAccounts: unshieldTransaction.public.accounts,
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      relayerRecipientSol: lightProvider.relayer.accounts.relayerRecipientSol,
      signer: lightProvider.relayer.accounts.relayerPubkey,
    });
    // createSolanaInstructionsWithAccounts
    const instructions = await createSolanaInstructions({
      action: unshieldTransaction.action,
      rootIndex,
      systemProof,
      remainingSolanaAccounts,
      accounts,
      publicTransactionVariables: unshieldTransaction.public,
      systemPspIdl: verifierIdl,
    });
    const transactionTester = new TestTransaction({
      transaction: unshieldTransaction,
      accounts,
      provider: lightProvider,
    });
    await transactionTester.getTestValues();
    const signatures =
      await lightProvider.relayer.sendAndConfirmSolanaInstructions(
        instructions,
        provider.connection!,
        { commitment: "finalized" },
        undefined,
        lightProvider,
      );
    console.log("SIGS UNSHIELD", signatures);

    await transactionTester.checkBalances(
      { publicInputs: systemProof.parsedPublicInputsObject },
      remainingSolanaAccounts,
      systemProofInputs,
      ACCOUNT,
    );
  };
});
