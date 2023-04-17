import * as anchor from "@coral-xyz/anchor";
import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
} from "@solana/web3.js";
const solana = require("@solana/web3.js");
import { assert } from "chai";

const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");
import { SPL_NOOP_ADDRESS } from "@solana/spl-account-compression";

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
  MERKLE_TREE_KEY,
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MINT,
  Provider,
  KEYPAIR_PRIVKEY,
  AUTHORITY_ONE,
  newAccountWithTokens,
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
  strToArr,
  RECIPIENT_TOKEN_ACCOUNT,
  TOKEN_REGISTRY,
  Action,
  TestRelayer,
  getAccountUtxos,
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";
import { Account } from "light-sdk/lib/account";

var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER;
var KEYPAIR;
var deposit_utxo1;

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
  console.log("merkleTreeProgram: ", merkleTreeProgramId.toBase58());
  const merkleTreeProgram: anchor.Program<MerkleTreeProgram> =
    new anchor.Program(IDL_MERKLE_TREE_PROGRAM, merkleTreeProgramId);

  const msg = Buffer.alloc(877).fill(1);
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

  const userKeypair = ADMIN_AUTH_KEYPAIR; //new SolanaKeypair();
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
      pubkey: MERKLE_TREE_KEY,
      poseidon,
    });
    console.log(merkleTree);
  });

  // TODO(vadorovsky): We probably need some parts of that test to the SDK.
  it("shielded transfer 1 & close", async () => {
    let balance = await provider.connection.getBalance(
      verifierState,
      "confirmed",
    );
    if (balance === 0) {
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(verifierState, 1_000_000_000),
        "confirmed",
      );
    }

    let tx0 = await verifierProgram.methods
      .shieldedTransferFirst(msg)
      .accounts({
        signingAddress: ADMIN_AUTH_KEYPAIR.publicKey,
        systemProgram: solana.SystemProgram.programId,
        verifierState: verifierState,
      })
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc(confirmConfig);

    console.log(tx0);

    let verifierAcc = await verifierProgram.account.verifierState.fetch(
      verifierState,
      "confirmed",
    );
    assert.equal(verifierAcc.msg.toString(), msg.toString());

    let tx1 = await verifierProgram.methods
      .shieldedTransferClose()
      .accounts({
        signingAddress: ADMIN_AUTH_KEYPAIR.publicKey,
        verifierState: verifierState,
      })
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc(confirmConfig);

    console.log(tx1);

    let accountInfo = await provider.connection.getAccountInfo(
      verifierState,
      "confirmed",
    );
    assert.equal(accountInfo, null);
  });

  it("shielded transfer 1 & 2", async () => {
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(verifierState, 1_000_000_000),
      "confirmed",
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        ADMIN_AUTH_KEYPAIR.publicKey,
        1_000_000_000,
      ),
      "confirmed",
    );

    for (var i = 0; i < 2; i++) {
      let msg_i = Buffer.alloc(877).fill(i);
      let tx = await verifierProgram.methods
        .shieldedTransferFirst(msg_i)
        .accounts({
          signingAddress: ADMIN_AUTH_KEYPAIR.publicKey,
          systemProgram: solana.SystemProgram.programId,
          verifierState: verifierState,
        })
        .signers([ADMIN_AUTH_KEYPAIR])
        .rpc({
          commitment: "confirmed",
        });

      console.log("tx" + i + ": " + tx);
    }

    let tx = await verifierProgram.methods
      .shieldedTransferSecond()
      .accounts({
        signingAddress: ADMIN_AUTH_KEYPAIR.publicKey,
        verifierState: verifierState,
        logWrapper: SPL_NOOP_ADDRESS,
      })
      .signers([ADMIN_AUTH_KEYPAIR])
      .rpc(confirmConfig);

    console.log(tx);

    let accountInfo = await provider.connection.getAccountInfo(
      verifierState,
      "confirmed",
    );
    assert.equal(accountInfo, null);
  });

  it("Deposit 10 utxo", async () => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }

    let balance = await provider.connection.getBalance(
      Transaction.getSignerAuthorityPda(
        merkleTreeProgram.programId,
        verifierProgramOneProgramId,
      ),
      "confirmed",
    );
    if (balance === 0) {
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(
          Transaction.getSignerAuthorityPda(
            merkleTreeProgram.programId,
            verifierProgramOneProgramId,
          ),
          1_000_000_000,
        ),
        "confirmed",
      );
    }

    for (var i = 0; i < 1; i++) {
      console.log("Deposit with 10 utxos ", i);

      let depositAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);
      let depositFeeAmount = 10_000 + Math.floor(Math.random() * 1_000_000_000);

      await token.approve(
        provider.connection,
        ADMIN_AUTH_KEYPAIR,
        userTokenAccount,
        AUTHORITY_ONE, //delegate
        USER_TOKEN_ACCOUNT, // owner
        depositAmount * 2,
        [USER_TOKEN_ACCOUNT],
      );
      const lightProvider = await Provider.init({
        wallet: ADMIN_AUTH_KEYPAIR,
        relayer: RELAYER,
      });

      let deposit_utxo1 = new Utxo({
        poseidon: POSEIDON,
        assets: [FEE_ASSET, MINT],
        amounts: [
          new anchor.BN(depositFeeAmount),
          new anchor.BN(depositAmount),
        ],
        account: KEYPAIR,
      });

      let txParams = new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: MERKLE_TREE_KEY,
        senderSpl: userTokenAccount,
        senderSol: ADMIN_AUTH_KEYPAIR.publicKey,
        verifier: new VerifierOne(),
        poseidon: POSEIDON,
        lookUpTable: LOOK_UP_TABLE,
        action: Action.SHIELD,
        transactionIndex: 0,
      });
      let tx = new Transaction({
        provider: lightProvider,
        params: txParams,
      });
      await tx.compileAndProve();

      try {
        let res = await tx.sendAndConfirmTransaction();
        console.log(res);
      } catch (e) {
        console.log(e);
      }
      await tx.checkBalances(KEYPAIR);
      // uncomment below if not running the "deposit" test
    }
  });

  it("Deposit", async () => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }

    let depositAmount =
      10_000 + (Math.floor(Math.random() * 1_000_000_000) % 1_100_000_000);
    let depositFeeAmount =
      10_000 + (Math.floor(Math.random() * 1_000_000_000) % 1_100_000_000);
    try {
      await token.approve(
        provider.connection,
        ADMIN_AUTH_KEYPAIR,
        userTokenAccount,
        AUTHORITY, //delegate
        USER_TOKEN_ACCOUNT, // owner
        depositAmount * 2,
        [USER_TOKEN_ACCOUNT],
      );
      console.log("approved");
    } catch (error) {
      console.log(error);
    }

    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });

    for (var i = 0; i < 1; i++) {
      console.log("Deposit ", i);
      deposit_utxo1 = new Utxo({
        poseidon: POSEIDON,
        assets: [FEE_ASSET, MINT],
        amounts: [
          new anchor.BN(depositFeeAmount),
          new anchor.BN(depositAmount),
        ],
        account: KEYPAIR,
      });

      let txParams = new TransactionParameters({
        outputUtxos: [deposit_utxo1],
        merkleTreePubkey: MERKLE_TREE_KEY,
        senderSpl: userTokenAccount,
        senderSol: ADMIN_AUTH_KEYPAIR.publicKey,
        verifier: new VerifierZero(),
        lookUpTable: LOOK_UP_TABLE,
        action: Action.SHIELD,
        poseidon: POSEIDON,
        transactionIndex: 1,
      });
      let tx = new Transaction({
        provider: lightProvider,
        params: txParams,
      });
      await tx.compileAndProve();

      try {
        let res = await tx.sendAndConfirmTransaction();
        console.log(res);
      } catch (e) {
        console.log("erorr here  ------------------------->", e);
        console.log("AUTHORITY: ", AUTHORITY.toBase58());
      }
      await tx.checkBalances(KEYPAIR);
    }
    await lightProvider.relayer.updateMerkleTree(lightProvider);
  });

  it("Withdraw", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let merkleTree = await SolMerkleTree.build({
      pubkey: MERKLE_TREE_KEY,
      poseidon,
    });

    let leavesPdas = await SolMerkleTree.getInsertedLeaves(MERKLE_TREE_KEY);

    let { decryptedUtxos } = await getAccountUtxos({
      leavesPdas,
      provider,
      account: KEYPAIR,
      poseidon: POSEIDON,
      transactionIndex: 0,
      aes: true,
      merkleTreePdaPublicKey: MERKLE_TREE_KEY,
    });

    const origin = new anchor.web3.Account();
    var tokenRecipient = recipientTokenAccount;

    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });

    let txParams = new TransactionParameters({
      inputUtxos: [decryptedUtxos[0]],
      merkleTreePubkey: MERKLE_TREE_KEY,
      recipientSpl: tokenRecipient,
      recipientSol: origin.publicKey,
      verifier: new VerifierZero(),
      relayer: RELAYER,
      action: Action.UNSHIELD,
      poseidon,
      transactionIndex: 2,
    });
    let tx = new Transaction({
      provider: lightProvider,
      // relayer,
      // payer: ADMIN_AUTH_KEYPAIR,
      shuffleEnabled: false,
      params: txParams,
    });

    await tx.compileAndProve();

    // TODO: add check in client to avoid rent exemption issue
    // add enough funds such that rent exemption is ensured
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        RELAYER.accounts.relayerRecipientSol,
        1_000_000,
      ),
      "confirmed",
    );
    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log("AUTHORITY: ", AUTHORITY.toBase58());
    }
    await tx.checkBalances();
  });

  it("Withdraw 10 utxos", async () => {
    POSEIDON = await circomlibjs.buildPoseidonOpt();

    let mtFetched = await merkleTreeProgram.account.transactionMerkleTree.fetch(
      MERKLE_TREE_KEY,
    );

    let merkleTree = await SolMerkleTree.build({
      pubkey: MERKLE_TREE_KEY,
      poseidon: POSEIDON,
    });

    let leavesPdas = await SolMerkleTree.getInsertedLeaves(MERKLE_TREE_KEY);

    let { decryptedUtxos } = await getAccountUtxos({
      leavesPdas,
      provider,
      account: KEYPAIR,
      poseidon: POSEIDON,
      transactionIndex: 0,
      aes: true,
      merkleTreePdaPublicKey: MERKLE_TREE_KEY,
    });

    let inputUtxos: Utxo[] = [];
    inputUtxos.push(decryptedUtxos[0]);

    const relayerRecipientSol = SolanaKeypair.generate().publicKey;
    const recipientSol = SolanaKeypair.generate().publicKey;
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });

    await lightProvider.provider!.connection.confirmTransaction(
      await lightProvider.provider!.connection.requestAirdrop(
        relayerRecipientSol,
        1_000_000,
      ),
    );
    await lightProvider.provider!.connection.confirmTransaction(
      await lightProvider.provider!.connection.requestAirdrop(
        recipientSol,
        1_000_000,
      ),
    );

    let txParams = new TransactionParameters({
      inputUtxos,
      outputUtxos: [
        new Utxo({
          poseidon: POSEIDON,
          assets: inputUtxos[0].assets,
          amounts: [new BN(0), inputUtxos[0].amounts[1]],
        }),
      ],

      // outputUtxos: [new Utxo({poseidon: POSEIDON, assets: inputUtxos[0].assets, amounts: [inputUtxos[0].amounts[0], new BN(0)]})],
      merkleTreePubkey: MERKLE_TREE_KEY,
      recipientSpl: recipientTokenAccount,
      recipientSol,
      verifier: new VerifierOne(),
      relayer: RELAYER,
      poseidon: POSEIDON,
      action: Action.UNSHIELD,
      transactionIndex: 3,
    });
    let tx = new Transaction({
      provider: lightProvider,
      // relayer,
      params: txParams,
    });

    await tx.compileAndProve();

    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
    }
    await tx.checkBalances();
  });
});
