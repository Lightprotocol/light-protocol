import * as anchor from "@coral-xyz/anchor";
import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
} from "@solana/web3.js";
const solana = require("@solana/web3.js");
import _ from "lodash";
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
  getUnspentUtxo,
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
  USER_TOKEN_ACCOUNT,
  createTestAccounts,
  userTokenAccount,
  recipientTokenAccount,
  FEE_ASSET,
  confirmConfig,
  TransactionParameters,
  Relayer,
  verifierProgramOneProgramId,
  SolMerkleTree,
  updateMerkleTreeForTest,
  IDL_MERKLE_TREE_PROGRAM,
  verifierStorageProgramId,
  User,
  IDL_VERIFIER_PROGRAM,
  strToArr,
} from "light-sdk";

import { BN } from "@coral-xyz/anchor";
import { Account } from "light-sdk/lib/account";

var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER_RECIPIENT;
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

  const msg = Buffer.alloc(887).fill(1);
  const verifierProgram = new anchor.Program(
    IDL_VERIFIER_PROGRAM,
    verifierStorageProgramId,
  );
  const [verifierState] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      ADMIN_AUTH_KEYPAIR.publicKey.toBuffer(),
      anchor.utils.bytes.utf8.encode("VERIFIER_STATE"),
    ],
    verifierProgram.programId,
  );

  const userKeypair = new SolanaKeypair();

  before("init test setup Merkle tree lookup table etc ", async () => {
    await createTestAccounts(provider.connection);
    LOOK_UP_TABLE = await initLookUpTableFromFile(provider);
    await setUpMerkleTree(provider);

    POSEIDON = await circomlibjs.buildPoseidonOpt();

    KEYPAIR = new Account({
      poseidon: POSEIDON,
      seed: KEYPAIR_PRIVKEY.toString(),
    });
    RELAYER_RECIPIENT = new anchor.web3.Account().publicKey;
  });

  it.skip("build compressed merkle tree", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    // await updateMerkleTreeForTest(provider);
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

    console.log(verifierState);

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

    let tx1 = await verifierProgram.methods
      .shieldedTransferSecond()
      .accounts({
        signingAddress: ADMIN_AUTH_KEYPAIR.publicKey,
        verifierState: verifierState,
        logWrapper: SPL_NOOP_ADDRESS,
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
      const lightProvider = await Provider.native(ADMIN_AUTH_KEYPAIR);

      let tx = new Transaction({
        provider: lightProvider,
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
        sender: userTokenAccount,
        senderFee: ADMIN_AUTH_KEYPAIR.publicKey,
        verifier: new VerifierOne(),
      });
      await tx.compileAndProve(txParams);

      try {
        let res = await tx.sendAndConfirmTransaction();
        console.log(res);
      } catch (e) {
        console.log(e);
      }
      await tx.checkBalances(KEYPAIR);
      // uncomment below if not running the "deposit" test
      // await updateMerkleTreeForTest(provider);
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

    for (var i = 0; i < 1; i++) {
      console.log("Deposit ", i);

      const lightProvider = await Provider.native(ADMIN_AUTH_KEYPAIR);

      let tx = new Transaction({
        provider: lightProvider,
      });

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
        sender: userTokenAccount,
        senderFee: ADMIN_AUTH_KEYPAIR.publicKey,
        verifier: new VerifierZero(),
      });
      await tx.compileAndProve(txParams);

      try {
        let res = await tx.sendAndConfirmTransaction();
        console.log(res);
      } catch (e) {
        console.log(e);
        console.log("AUTHORITY: ", AUTHORITY.toBase58());
      }
      await tx.checkBalances(KEYPAIR);
    }
    await updateMerkleTreeForTest(provider);
  });

  it("Withdraw", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let merkleTree = await SolMerkleTree.build({
      pubkey: MERKLE_TREE_KEY,
      poseidon,
    });

    let leavesPdas = await SolMerkleTree.getInsertedLeaves(MERKLE_TREE_KEY);

    let decryptedUtxo1 = await getUnspentUtxo(
      leavesPdas,
      provider,
      KEYPAIR,
      POSEIDON,
      merkleTreeProgram,
      merkleTree.merkleTree,
      0,
    );

    const origin = new anchor.web3.Account();
    var tokenRecipient = recipientTokenAccount;

    const lightProvider = await Provider.native(ADMIN_AUTH_KEYPAIR);

    let relayer = new Relayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      lightProvider.lookUpTable,
      SolanaKeypair.generate().publicKey,
      new BN(100000),
    );

    let tx = new Transaction({
      provider: lightProvider,
      // relayer,
      // payer: ADMIN_AUTH_KEYPAIR,
      // shuffleEnabled: false,
    });

    let txParams = new TransactionParameters({
      inputUtxos: [decryptedUtxo1],
      merkleTreePubkey: MERKLE_TREE_KEY,
      recipient: tokenRecipient,
      recipientFee: origin.publicKey,
      verifier: new VerifierZero(),
      relayer,
    });

    await tx.compileAndProve(txParams);

    // TODO: add check in client to avoid rent exemption issue
    // add enough funds such that rent exemption is ensured
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        relayer.accounts.relayerRecipient,
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

    let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(
      MERKLE_TREE_KEY,
    );

    let merkleTree = await SolMerkleTree.build({
      pubkey: MERKLE_TREE_KEY,
      poseidon: POSEIDON,
    });

    let leavesPdas = await SolMerkleTree.getInsertedLeaves(MERKLE_TREE_KEY);

    let decryptedUtxo1 = await getUnspentUtxo(
      leavesPdas,
      provider,
      KEYPAIR,
      POSEIDON,
      merkleTreeProgram,
      merkleTree.merkleTree,
      0,
    );

    let inputUtxos = [];
    inputUtxos.push(decryptedUtxo1);

    const relayerRecipient = SolanaKeypair.generate().publicKey;
    const recipientFee = SolanaKeypair.generate().publicKey;
    const lightProvider = await Provider.native(ADMIN_AUTH_KEYPAIR);

    await lightProvider.provider.connection.confirmTransaction(
      await lightProvider.provider.connection.requestAirdrop(
        relayerRecipient,
        1_000_000,
      ),
    );
    await lightProvider.provider.connection.confirmTransaction(
      await lightProvider.provider.connection.requestAirdrop(recipientFee, 1_000_000),
    );

    let relayer = new Relayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      lightProvider.lookUpTable,
      relayerRecipient,
      new BN(100000),
    );

    let tx = new Transaction({
      provider: lightProvider,
      // relayer,
    });

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
      recipient: recipientTokenAccount,
      recipientFee,
      verifier: new VerifierOne(),
      relayer,
    });
    await tx.compileAndProve(txParams);

    try {
      let res = await tx.sendAndConfirmTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
    }
    await tx.checkBalances();
  });

  it("(user class) shield SOL", async () => {
    let amount = 3;
    let token = "SOL";
    const provider = await Provider.native(userKeypair);
    let res = await provider.provider.connection.requestAirdrop(
      userKeypair.publicKey,
      4_000_000_000,
    );
    await provider.provider.connection.confirmTransaction(res, "confirmed");
    const user = await User.load(provider);
    await user.shield({ amount, token });
    // TODO: add random amount and amount checks
  });
  it("(user class) transfer SOL", async () => {
    let amount = 1;
    let token = "SOL";
    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";

    const recipient = new anchor.BN(shieldedRecipient, "hex");
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);
    const provider = await Provider.native(userKeypair);
    const user = await User.load(provider);
    await user.transfer({
      amount,
      token,
      recipient,
      recipientEncryptionPublicKey, // TODO: do shielded address
    });

    // TODO: add random amount, recipient and amount checks
  });

  it("(user class) unshield SOL", async () => {
    let amount = 1;
    let token = "SOL";
    let recipient = new PublicKey(
      "E7jqevikamCMCda8yCsfNawj57FSotUZuref9MLZpWo1",
    );
    const provider = await Provider.native(userKeypair);
    const user = await User.load(provider);
    await user.unshield({ amount, token, recipient });
    // TODO: add random amount and amount checks
  });
});
