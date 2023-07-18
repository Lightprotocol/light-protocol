import * as anchor from "@coral-xyz/anchor";
import {
  Keypair,
  Keypair as SolanaKeypair,
  SystemProgram,
} from "@solana/web3.js";
import { Idl } from "@coral-xyz/anchor";

const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");

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
  IDL_VERIFIER_PROGRAM_ZERO,
  IDL_VERIFIER_PROGRAM_ONE,
  IDL_VERIFIER_PROGRAM_STORAGE,
  Account,
  airdropSol,
  MerkleTreeConfig,
} from "@lightprotocol/zk.js";

import { BN } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

var POSEIDON;
var RELAYER;
var KEYPAIR;

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

  before("init test setup Merkle tree lookup table etc ", async () => {
    await createTestAccounts(provider.connection, userTokenAccount);

    POSEIDON = await circomlibjs.buildPoseidonOpt();
    const seed = bs58.encode(new Uint8Array(32).fill(1));
    KEYPAIR = new Account({
      poseidon: POSEIDON,
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
      relayerFee: new BN(100_000),
      payer: ADMIN_AUTH_KEYPAIR,
    });
  });

  const performDeposit = async ({
    delegate,
    spl = false,
    message,
    senderSpl,
    shuffleEnabled = true,
    verifierIdl,
  }: {
    delegate: anchor.web3.PublicKey;
    spl: boolean;
    message?: Buffer;
    senderSpl: anchor.web3.PublicKey;
    shuffleEnabled: boolean;
    verifierIdl: Idl;
  }) => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }

    let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
    let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

    await token.approve(
      provider.connection,
      ADMIN_AUTH_KEYPAIR,
      userTokenAccount,
      delegate, // delegate
      USER_TOKEN_ACCOUNT, // owner
      depositAmount * 2,
      [USER_TOKEN_ACCOUNT],
    );
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });

    let deposit_utxo1 = spl
      ? new Utxo({
          poseidon: POSEIDON,
          assets: [FEE_ASSET, MINT],
          amounts: [
            new anchor.BN(depositFeeAmount),
            new anchor.BN(depositAmount),
          ],
          account: KEYPAIR,
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
        })
      : new Utxo({
          poseidon: POSEIDON,
          amounts: [new anchor.BN(depositFeeAmount)],
          account: KEYPAIR,
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
        });

    let txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      message,
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      senderSpl,
      senderSol: ADMIN_AUTH_KEYPAIR.publicKey,
      action: Action.SHIELD,
      poseidon: POSEIDON,
      verifierIdl: verifierIdl,
    });
    let transactionTester = new TestTransaction({
      txParams,
      provider: lightProvider,
    });
    await transactionTester.getTestValues();
    let tx = new Transaction({
      provider: lightProvider,
      params: txParams,
      shuffleEnabled,
    });
    await tx.compileAndProve();

    try {
      let res = await tx.sendAndConfirmTransaction();
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
  };

  it("Deposit (verifier one)", async () => {
    await performDeposit({
      delegate: AUTHORITY_ONE,
      spl: true,
      senderSpl: userTokenAccount,
      shuffleEnabled: true,
      verifierIdl: IDL_VERIFIER_PROGRAM_ONE,
    });
  });

  it("Deposit (verifier storage)", async () => {
    await performDeposit({
      delegate: AUTHORITY,
      spl: false,
      message: Buffer.alloc(900).fill(1),
      senderSpl: null,
      shuffleEnabled: false,
      verifierIdl: IDL_VERIFIER_PROGRAM_STORAGE,
    });
  });

  it("Deposit (verifier zero)", async () => {
    await performDeposit({
      delegate: AUTHORITY,
      spl: true,
      senderSpl: userTokenAccount,
      shuffleEnabled: true,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });
    await lightProvider.relayer.updateMerkleTree(lightProvider);
  });

  const performWithdrawal = async ({
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
    let user = await User.init({
      provider: lightProvider,
      account: KEYPAIR,
    });

    const origin = Keypair.generate();
    await airdropSol({
      provider: lightProvider.provider,
      lamports: 1000 * 1e9,
      recipientPublicKey: origin.publicKey,
    });

    let txParams = new TransactionParameters({
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
    });

    let transactionTester = new TestTransaction({
      txParams,
      provider: lightProvider,
    });
    await transactionTester.getTestValues();

    let tx = new Transaction({
      provider: lightProvider,
      shuffleEnabled,
      params: txParams,
    });

    await tx.compileAndProve();

    await tx.sendAndConfirmTransaction();

    await transactionTester.checkBalances(
      tx.transactionInputs,
      tx.remainingAccounts,
      tx.proofInput,
    );
  };

  it("Withdraw (verifier zero)", async () => {
    await performWithdrawal({
      outputUtxos: [],
      tokenProgram: MINT,
      recipientSpl: recipientTokenAccount,
      shuffleEnabled: false,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
  });

  it("Withdraw (verifier storage)", async () => {
    await performWithdrawal({
      outputUtxos: [],
      tokenProgram: SystemProgram.programId,
      message: Buffer.alloc(900).fill(1),
      shuffleEnabled: false,
      verifierIdl: IDL_VERIFIER_PROGRAM_STORAGE,
    });
  });

  it("Withdraw (verifier one)", async () => {
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });
    let user: User = await User.init({
      provider: lightProvider,
      account: KEYPAIR,
    });
    let inputUtxos: Utxo[] = [
      user.balance.tokenBalances.get(MINT.toBase58()).utxos.values().next()
        .value,
    ];
    await performWithdrawal({
      outputUtxos: [
        new Utxo({
          poseidon: POSEIDON,
          assets: inputUtxos[0].assets,
          amounts: [new BN(0), inputUtxos[0].amounts[1]],
          assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            lightProvider.lookUpTables.verifierProgramLookupTable,
        }),
      ],
      tokenProgram: MINT,
      recipientSpl: recipientTokenAccount,
      shuffleEnabled: true,
      verifierIdl: IDL_VERIFIER_PROGRAM_ONE,
    });
  });
});
