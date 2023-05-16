import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair, SystemProgram } from "@solana/web3.js";
import { Idl } from "@coral-xyz/anchor";

const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");

// TODO: add and use namespaces in SDK
import {
  Transaction,
  Utxo,
  LOOK_UP_TABLE,
  initLookUpTableFromFile,
  MerkleTreeProgram,
  merkleTreeProgramId,
  TRANSACTION_MERKLE_TREE_KEY,
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MINT,
  Provider,
  KEYPAIR_PRIVKEY,
  AUTHORITY_ONE,
  USER_TOKEN_ACCOUNT,
  createTestAccounts,
  userTokenAccount,
  recipientTokenAccount,
  FEE_ASSET,
  confirmConfig,
  TransactionParameters,
  IDL_MERKLE_TREE_PROGRAM,
  verifierStorageProgramId,
  User,
  Action,
  TestRelayer,
  TestTransaction,
  MESSAGE_MERKLE_TREE_KEY,
  IDL_VERIFIER_PROGRAM_ZERO,
  IDL_VERIFIER_PROGRAM_ONE,
  IDL_VERIFIER_PROGRAM_STORAGE,
} from "light-sdk";

import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
import { Account } from "light-sdk/lib/account";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

var POSEIDON;
var RELAYER;
var KEYPAIR;
var deposit_utxo1: Utxo;
var TRANSACTION_NONCE = 0;
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

    RELAYER = await new TestRelayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      LOOK_UP_TABLE,
      relayerRecipientSol,
      new BN(100_000),
    );
  });

  async function airdrop() {
    let balance = await provider.connection.getBalance(
      Transaction.getSignerAuthorityPda(
        merkleTreeProgram.programId,
        verifierStorageProgramId,
      ),
      "confirmed",
    );
    if (balance === 0) {
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(
          Transaction.getSignerAuthorityPda(
            merkleTreeProgram.programId,
            verifierStorageProgramId,
          ),
          1_000_000_000,
        ),
        "confirmed",
      );
    }
  }

  const performDeposit = async ({
    delegate,
    spl = false,
    message,
    messageMerkleTreePubkey,
    senderSpl,
    transactionNonce,
    shuffleEnabled = true,
    verifierIdl,
  }: {
    delegate: anchor.web3.PublicKey;
    spl: boolean;
    message?: Buffer;
    messageMerkleTreePubkey?: anchor.web3.PublicKey;
    senderSpl: anchor.web3.PublicKey;
    transactionNonce: number;
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
        })
      : new Utxo({
          poseidon: POSEIDON,
          amounts: [new anchor.BN(depositFeeAmount)],
          account: KEYPAIR,
        });

    let txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      message,
      messageMerkleTreePubkey,
      transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
      senderSpl,
      senderSol: ADMIN_AUTH_KEYPAIR.publicKey,
      lookUpTable: LOOK_UP_TABLE,
      action: Action.SHIELD,
      poseidon: POSEIDON,
      transactionNonce,
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
    TRANSACTION_NONCE++;
  };

  it("Deposit (verifier one)", async () => {
    await performDeposit({
      delegate: AUTHORITY_ONE,
      spl: true,
      senderSpl: userTokenAccount,
      transactionNonce: TRANSACTION_NONCE,
      shuffleEnabled: true,
      verifierIdl: IDL_VERIFIER_PROGRAM_ONE,
    });
  });

  it("Deposit (verifier storage)", async () => {
    await performDeposit({
      delegate: AUTHORITY,
      spl: false,
      message: Buffer.alloc(900).fill(1),
      messageMerkleTreePubkey: MESSAGE_MERKLE_TREE_KEY,
      senderSpl: null,
      transactionNonce: TRANSACTION_NONCE,
      shuffleEnabled: false,
      verifierIdl: IDL_VERIFIER_PROGRAM_STORAGE,
    });
  });

  it("Deposit (verifier zero)", async () => {
    await performDeposit({
      delegate: AUTHORITY,
      spl: true,
      senderSpl: userTokenAccount,
      transactionNonce: TRANSACTION_NONCE,
      shuffleEnabled: true,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });
    await lightProvider.relayer.updateMerkleTree(lightProvider);
  });

  const performWithdrawal = async ({
    outputUtxos,
    tokenProgram,
    message,
    messageMerkleTreePubkey,
    recipientSpl,
    transactionNonce,
    shuffleEnabled = true,
    verifierIdl,
  }: {
    outputUtxos: Array<Utxo>;
    tokenProgram: anchor.web3.PublicKey;
    message?: Buffer;
    messageMerkleTreePubkey?: anchor.web3.PublicKey;
    recipientSpl?: anchor.web3.PublicKey;
    transactionNonce: number;
    shuffleEnabled: boolean;
    verifierIdl: Idl;
  }) => {
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });
    let user = await User.init({
      provider: lightProvider,
      account: KEYPAIR,
    });

    const origin = new anchor.web3.Account();

    let txParams = new TransactionParameters({
      inputUtxos: [
        user.balance.tokenBalances
          .get(tokenProgram.toBase58())
          .utxos.values()
          .next().value,
      ],
      outputUtxos,
      message,
      messageMerkleTreePubkey,
      transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
      recipientSpl,
      recipientSol: origin.publicKey,
      relayer: RELAYER,
      action: Action.UNSHIELD,
      poseidon: POSEIDON,
      transactionNonce,
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
    TRANSACTION_NONCE++;
  };

  it("Withdraw (verifier zero)", async () => {
    await performWithdrawal({
      outputUtxos: [],
      tokenProgram: MINT,
      recipientSpl: recipientTokenAccount,
      transactionNonce: TRANSACTION_NONCE,
      shuffleEnabled: false,
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });
  });

  it("Withdraw (verifier storage)", async () => {
    await performWithdrawal({
      outputUtxos: [],
      tokenProgram: SystemProgram.programId,
      message: Buffer.alloc(900).fill(1),
      messageMerkleTreePubkey: MESSAGE_MERKLE_TREE_KEY,
      transactionNonce: TRANSACTION_NONCE,
      shuffleEnabled: false,
      verifierIdl: IDL_VERIFIER_PROGRAM_STORAGE,
    });
  });

  it("Withdraw (verifier one)", async () => {
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
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
        }),
      ],
      tokenProgram: MINT,
      recipientSpl: recipientTokenAccount,
      transactionNonce: TRANSACTION_NONCE,
      shuffleEnabled: true,
      verifierIdl: IDL_VERIFIER_PROGRAM_ONE,
    });
  });
});
