import * as anchor from "@coral-xyz/anchor";
import {
  Keypair,
  Keypair as SolanaKeypair,
  SystemProgram,
} from "@solana/web3.js";
import { Idl } from "@coral-xyz/anchor";
const token = require("@solana/spl-token");
const circomlibjs = require("circomlibjs");

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
  USER_TOKEN_ACCOUNT,
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
} from "../../src";

import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

let POSEIDON: any;
let RELAYER: TestRelayer;
let KEYPAIR: Account;

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

    POSEIDON = await circomlibjs.buildPoseidonOpt();
    const seed = bs58.encode(new Uint8Array(32).fill(1));
    KEYPAIR = new Account({
      poseidon: POSEIDON,
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
      senderSpl: userTokenAccount,
      shuffleEnabled: true,
      verifierIdl: IDL_LIGHT_PSP10IN2OUT,
    });
  });

  it("Shield (verifier storage)", async () => {
    await performShield({
      delegate: AUTHORITY,
      spl: false,
      message: Buffer.alloc(900).fill(1),
      senderSpl: null,
      shuffleEnabled: false,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT_STORAGE,
    });
  });

  it("Shield (verifier zero)", async () => {
    await performShield({
      delegate: AUTHORITY,
      spl: true,
      senderSpl: userTokenAccount,
      shuffleEnabled: true,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT,
      updateMerkleTree: true,
    });
  });

  it("Shield (verifier zero)", async () => {
    await performUnshield({
      outputUtxos: [],
      tokenProgram: MINT,
      recipientSpl: recipientTokenAccount,
      shuffleEnabled: false,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT,
    });
  });

  it("Shield (verifier storage)", async () => {
    await performUnshield({
      outputUtxos: [],
      tokenProgram: SystemProgram.programId,
      message: Buffer.alloc(900).fill(1),
      shuffleEnabled: false,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT_STORAGE,
    });
  });

  it("Shield (verifier one)", async () => {
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });
    const user: User = await User.init({
      provider: lightProvider,
      account: KEYPAIR,
    });
    const inputUtxos: Utxo[] = [
      user.balance.tokenBalances.get(MINT.toBase58()).utxos.values().next()
        .value,
    ];
    await performUnshield({
      outputUtxos: [
        new Utxo({
          poseidon: POSEIDON,
          publicKey: inputUtxos[0].publicKey,
          assets: inputUtxos[0].assets,
          amounts: [BN_0, inputUtxos[0].amounts[1]],
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
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
    senderSpl,
    shuffleEnabled = true,
    verifierIdl,
    updateMerkleTree = false,
  }: {
    delegate: anchor.web3.PublicKey;
    spl: boolean;
    message?: Buffer;
    senderSpl: anchor.web3.PublicKey;
    shuffleEnabled: boolean;
    verifierIdl: Idl;
    updateMerkleTree?: boolean;
  }) => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }

    const shieldAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
    const shieldFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

    await token.approve(
      provider.connection,
      ADMIN_AUTH_KEYPAIR,
      userTokenAccount,
      delegate, // delegate
      USER_TOKEN_ACCOUNT, // owner
      shieldAmount * 2,
      [USER_TOKEN_ACCOUNT],
    );
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });

    const shieldUtxo1 = spl
      ? new Utxo({
          poseidon: POSEIDON,
          assets: [FEE_ASSET, MINT],
          amounts: [
            new anchor.BN(shieldFeeAmount),
            new anchor.BN(shieldAmount),
          ],
          publicKey: KEYPAIR.pubkey,
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
        })
      : new Utxo({
          poseidon: POSEIDON,
          amounts: [new anchor.BN(shieldFeeAmount)],
          publicKey: KEYPAIR.pubkey,
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
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
      poseidon: POSEIDON,
      verifierIdl: verifierIdl,
      account: KEYPAIR,
    });
    const transactionTester = new TestTransaction({
      txParams,
      provider: lightProvider,
    });
    await transactionTester.getTestValues();
    const { rootIndex, remainingAccounts } = await lightProvider.getRootIndex();
    const tx = new Transaction({
      rootIndex,
      nextTransactionMerkleTree: remainingAccounts.nextTransactionMerkleTree,
      solMerkleTree: lightProvider.solMerkleTree!,
      params: txParams,
      shuffleEnabled,
    });
    const instructions = await tx.compileAndProve(
      lightProvider.poseidon,
      KEYPAIR,
    );

    try {
      const res = await lightProvider.sendAndConfirmTransaction(instructions);
      console.log(res);
    } catch (e) {
      console.log(e);
    }
    await transactionTester.checkBalances(
      tx.transactionInputs,
      tx.remainingAccounts,
      tx.proofInput,
      KEYPAIR,
    );

    if (updateMerkleTree) {
      await lightProvider.relayer.updateMerkleTree(lightProvider);
    }
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
      account: KEYPAIR,
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
          .get(tokenProgram.toBase58())
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
      poseidon: POSEIDON,
      verifierIdl: verifierIdl,
      account: KEYPAIR,
    });

    const transactionTester = new TestTransaction({
      txParams,
      provider: lightProvider,
    });
    await transactionTester.getTestValues();

    const { rootIndex, remainingAccounts } = await lightProvider.getRootIndex();
    const tx = new Transaction({
      rootIndex,
      nextTransactionMerkleTree: remainingAccounts.nextTransactionMerkleTree,
      solMerkleTree: lightProvider.solMerkleTree!,
      shuffleEnabled,
      params: txParams,
    });

    const instructions = await tx.compileAndProve(
      lightProvider.poseidon,
      KEYPAIR,
    );

    await lightProvider.sendAndConfirmShieldedTransaction(instructions);

    await transactionTester.checkBalances(
      tx.transactionInputs,
      tx.remainingAccounts,
      tx.proofInput,
      KEYPAIR,
    );
  };
});
