import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair, SystemProgram } from "@solana/web3.js";
const solana = require("@solana/web3.js");
import { assert } from "chai";

const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");

// TODO: add and use  namespaces in SDK
import {
  Transaction,
  VerifierZero,
  VerifierOne,
  Utxo,
  setUpMerkleTree,
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
  verifierProgramOneProgramId,
  SolMerkleTree,
  IDL_MERKLE_TREE_PROGRAM,
  verifierStorageProgramId,
  User,
  IDL_VERIFIER_PROGRAM_STORAGE,
  Action,
  TestRelayer,
  TestTransaction,
  MESSAGE_MERKLE_TREE_KEY,
  VerifierStorage,
  Verifier,
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";
import { Account } from "light-sdk/lib/account";

var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER;
var KEYPAIR;
var deposit_utxo1: Utxo;

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
  const merkleTreeProgram: anchor.Program<MerkleTreeProgram> =
    new anchor.Program(IDL_MERKLE_TREE_PROGRAM, merkleTreeProgramId);

  const verifierProgram = new anchor.Program(
    IDL_VERIFIER_PROGRAM_STORAGE,
    verifierStorageProgramId,
  );
  const [verifierState] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      ADMIN_AUTH_KEYPAIR.publicKey.toBuffer(),
      anchor.utils.bytes.utf8.encode("VERIFIER_STATE"),
    ],
    verifierProgram.programId,
  );

  const userKeypair = ADMIN_AUTH_KEYPAIR; 
  let userSplAccount = null;

  before("init test setup Merkle tree lookup table etc ", async () => {
    let initLog = console.log;
    // console.log = () => {};
    await createTestAccounts(provider.connection, userTokenAccount);
    LOOK_UP_TABLE = await initLookUpTableFromFile(provider);
    await setUpMerkleTree(provider);
    // console.log = initLog;
    POSEIDON = await circomlibjs.buildPoseidonOpt();

    KEYPAIR = new Account({
      poseidon: POSEIDON,
      seed: KEYPAIR_PRIVKEY.toString(),
    });

    const relayerRecipientSol = SolanaKeypair.generate().publicKey;

    await provider.connection.requestAirdrop(
      relayerRecipientSol,
      2_000_000_000,
    );

    RELAYER = await new TestRelayer(
      userKeypair.publicKey,
      LOOK_UP_TABLE,
      relayerRecipientSol,
      new BN(100000),
    );
  });

  it.skip("build compressed merkle tree", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let merkleTree = await SolMerkleTree.build({
      pubkey: TRANSACTION_MERKLE_TREE_KEY,
      poseidon,
    });
    console.log(merkleTree);
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
    verifier,
    transactionNonce,
    shuffleEnabled = true,
  }: {
    delegate: anchor.web3.PublicKey;
    spl: boolean;
    message?: Buffer;
    messageMerkleTreePubkey?: anchor.web3.PublicKey;
    senderSpl: anchor.web3.PublicKey;
    verifier: Verifier;
    transactionNonce: number;
    shuffleEnabled: boolean;
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
      verifier,
      lookUpTable: LOOK_UP_TABLE,
      action: Action.SHIELD,
      poseidon: POSEIDON,
      transactionNonce,
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
    await airdrop();
    await performDeposit({
      delegate: AUTHORITY_ONE,
      spl: true,
      senderSpl: userTokenAccount,
      verifier: new VerifierOne(),
      transactionNonce: 0,
      shuffleEnabled: true,
    });
  });

  it("Deposit (verifier storage)", async () => {
    await airdrop();
    await performDeposit({
      delegate: AUTHORITY,
      spl: false,
      message: Buffer.alloc(938).fill(1),
      messageMerkleTreePubkey: MESSAGE_MERKLE_TREE_KEY,
      senderSpl: null,
      verifier: new VerifierStorage(),
      transactionNonce: 1,
      shuffleEnabled: false,
    });
  });

  it("Deposit (verifier zero)", async () => {
    await airdrop();
    await performDeposit({
      delegate: AUTHORITY,
      spl: true,
      senderSpl: userTokenAccount,
      verifier: new VerifierZero(),
      transactionNonce: 2,
      shuffleEnabled: true,
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
    verifier,
    transactionNonce,
    shuffleEnabled = true,
  }: {
    outputUtxos: Array<Utxo>;
    tokenProgram: anchor.web3.PublicKey;
    message?: Buffer;
    messageMerkleTreePubkey?: anchor.web3.PublicKey;
    recipientSpl?: anchor.web3.PublicKey;
    verifier: Verifier;
    transactionNonce: number;
    shuffleEnabled: boolean;
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
      verifier,
      relayer: RELAYER,
      action: Action.UNSHIELD,
      poseidon: POSEIDON,
      transactionNonce,
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

    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
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
      verifier: new VerifierZero(),
      transactionNonce: 3,
      shuffleEnabled: false,
    });
  });

  it("Withdraw (verifier storage)", async () => {
    await performWithdrawal({
      outputUtxos: [],
      tokenProgram: SystemProgram.programId,
      message: Buffer.alloc(938).fill(1),
      messageMerkleTreePubkey: MESSAGE_MERKLE_TREE_KEY,
      verifier: new VerifierStorage(),
      transactionNonce: 4,
      shuffleEnabled: false,
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
      verifier: new VerifierOne(),
      transactionNonce: 5,
      shuffleEnabled: true,
    });
  });
});
