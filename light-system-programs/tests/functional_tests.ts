import * as anchor from "@coral-xyz/anchor";
import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
const solana = require("@solana/web3.js");
import _ from "lodash";
import { assert } from "chai";

const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");

// TODO: add and use  namespaces in SDK
import {
  Transaction,
  VerifierZero,
  VerifierOne,
  Keypair,
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
  LightInstance,
  Relayer,
  verifierProgramOneProgramId,
  SolMerkleTree,
  updateMerkleTreeForTest,
  IDL_MERKLE_TREE_PROGRAM,
  getLightInstance,
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
  process.env.ANCHOR_WALLET =
    "/Users/" + process.env.USER + "/.config/solana/id.json";

  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig
  );
  anchor.setProvider(provider);
  console.log("merkleTreeProgram: ", merkleTreeProgramId.toBase58());
  const merkleTreeProgram: anchor.Program<MerkleTreeProgram> =
    new anchor.Program(IDL_MERKLE_TREE_PROGRAM, merkleTreeProgramId);

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

  it("Deposit 10 utxo", async () => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }

    let balance = await provider.connection.getBalance(
      Transaction.getSignerAuthorityPda(
        merkleTreeProgram.programId,
        verifierProgramOneProgramId
      ),
      "confirmed"
    );
    if (balance === 0) {
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(
          Transaction.getSignerAuthorityPda(
            merkleTreeProgram.programId,
            verifierProgramOneProgramId
          ),
          1_000_000_000
        ),
        "confirmed"
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
        [USER_TOKEN_ACCOUNT]
      );
      const prov = await Provider.native(ADMIN_AUTH_KEYPAIR);

      let tx = new Transaction({
        provider: prov,
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
        payer: ADMIN_AUTH_KEYPAIR,
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
        [USER_TOKEN_ACCOUNT]
      );
      console.log("approved");
    } catch (error) {
      console.log(error);
    }

    for (var i = 0; i < 1; i++) {
      console.log("Deposit ", i);

      const prov = await Provider.native(ADMIN_AUTH_KEYPAIR);

      let tx = new Transaction({
        provider: prov,
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
        payer: ADMIN_AUTH_KEYPAIR,
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
      0
    );

    const origin = new anchor.web3.Account();
    var tokenRecipient = recipientTokenAccount;

    const prov = await Provider.native(ADMIN_AUTH_KEYPAIR);

    let relayer = new Relayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      prov.lookUpTable,
      SolanaKeypair.generate().publicKey,
      new BN(100000)
    );

    let tx = new Transaction({
      provider: prov,
      relayer,
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
      payer: ADMIN_AUTH_KEYPAIR,
    });

    await tx.compileAndProve(txParams);

    // await testTransaction({transaction: SHIELDED_TRANSACTION, deposit: false,provider, signer: ADMIN_AUTH_KEYPAIR, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA});

    // TODO: add check in client to avoid rent exemption issue
    // add enough funds such that rent exemption is ensured
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        relayer.accounts.relayerRecipient,
        1_000_000
      ),
      "confirmed"
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
      MERKLE_TREE_KEY
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
      0
    );

    let inputUtxos = [];
    inputUtxos.push(decryptedUtxo1);

    const relayerRecipient = SolanaKeypair.generate().publicKey;
    const recipientFee = SolanaKeypair.generate().publicKey;
    const prov = await Provider.native(ADMIN_AUTH_KEYPAIR);

    await prov.provider.connection.confirmTransaction(
      await prov.provider.connection.requestAirdrop(relayerRecipient, 1_000_000)
    );
    await prov.provider.connection.confirmTransaction(
      await prov.provider.connection.requestAirdrop(recipientFee, 1_000_000)
    );

    let relayer = new Relayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      prov.lookUpTable,
      relayerRecipient,
      new BN(100000)
    );

    let tx = new Transaction({
      provider: prov,
      relayer,
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
      payer: ADMIN_AUTH_KEYPAIR,
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
});
