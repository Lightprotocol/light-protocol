import * as anchor from "@project-serum/anchor";

import {
  Utxo,
  Transaction,
  ADMIN_AUTH_KEYPAIR,
  REGISTERED_POOL_PDA_SPL_TOKEN,
  initLookUpTableFromFile,
  setUpMerkleTree,
  createTestAccounts,
  KEYPAIR_PRIVKEY,
  Keypair,
  MERKLE_TREE_KEY,
  MINT,
  offerBurnerPrivkey,
  offerAuthorityPrivkey,
  bidderPrivkey,
  feeRecipient1Privkey,
  feeRecipientPrivkey,
  TransactionParameters,
  SolMerkleTree,
  LightInstance,
  userTokenAccount,
  USER_TOKEN_ACCOUNT,
  verifierProgramTwoProgramId,
  merkleTreeProgramId,
  VerifierZero,
  updateMerkleTreeForTest,
  ADMIN_AUTH_KEY,
  VerifierTwo,
  confirmConfig,
  Relayer
} from "light-sdk";
import {
  Keypair as SolanaKeypair,
  SystemProgram,
} from "@solana/web3.js";
import {
  MockVerifier
} from "../sdk/src/index";

import { buildPoseidonOpt } from "circomlibjs";
import { assert, expect } from "chai";
import { BN } from "@project-serum/anchor";
import { it } from "mocha";
const token = require("@solana/spl-token");
var POSEIDON,
  LOOK_UP_TABLE,
  KEYPAIR;

describe("Mock verifier functional", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = "/home/" + process.env.USER + "/.config/solana/id.json"
  
  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig
  );
  anchor.setProvider(provider);
  before( async () => {
    console.log("Initing accounts");

    await createTestAccounts(provider.connection);
    POSEIDON = await buildPoseidonOpt();
    KEYPAIR = new Keypair({
      poseidon: POSEIDON,
      seed: KEYPAIR_PRIVKEY.toString(),
    });
    await setUpMerkleTree(provider);
    LOOK_UP_TABLE = await initLookUpTableFromFile(provider);
  });

  var outputUtxo
  it("Test Deposit MockVerifier cpi VerifierTwo", async () => {
    const poseidon = await buildPoseidonOpt();

    let lightInstance: LightInstance = {
      solMerkleTree: new SolMerkleTree({ poseidon, pubkey: MERKLE_TREE_KEY }),
      lookUpTable: LOOK_UP_TABLE,
      provider
    };

    outputUtxo = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      keypair: new Keypair({poseidon, seed: new Array(32).fill(1).toString()}),
      amounts: [new BN(1_000_000)],
    });

    const txParams = new TransactionParameters({
      outputUtxos: [outputUtxo],
      merkleTreePubkey: MERKLE_TREE_KEY,
      sender: userTokenAccount, // just any token account
      senderFee: ADMIN_AUTH_KEY, //
      verifier: new VerifierTwo(),
    });

    const appParams = {
      verifier: new MockVerifier(),
      inputs: { },
    }

    let tx = new Transaction({
      instance: lightInstance,
      payer: ADMIN_AUTH_KEYPAIR,
      shuffleEnabled: false,
    });

    await tx.compile(txParams, appParams);
    await tx.instance.provider.connection.confirmTransaction(
      await tx.instance.provider.connection.requestAirdrop(tx.params.accounts.authority, 1_000_000_000, "confirmed")
    );
    await tx.getProof();
    await tx.getAppProof();
    await tx.sendAndConfirmTransaction();
    await updateMerkleTreeForTest(provider)
  });

  it("Test Withdrawal MockVerifier cpi VerifierTwo", async () => {
    const poseidon = await buildPoseidonOpt();

    let lightInstance: LightInstance = {
      solMerkleTree: await SolMerkleTree.build({pubkey: MERKLE_TREE_KEY, poseidon}),
      lookUpTable: LOOK_UP_TABLE,
      provider
    };
    const relayerRecipient = SolanaKeypair.generate().publicKey;
    let relayer = new Relayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      lightInstance.lookUpTable,
      relayerRecipient,
      new BN(100000)
  );
  await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(relayerRecipient, 10000000));

    // TODO: add check that recipients are defined if withdrawal
    const txParams = new TransactionParameters({
      inputUtxos: [outputUtxo],
      merkleTreePubkey: MERKLE_TREE_KEY,
      recipient: userTokenAccount, // just any token account
      recipientFee: ADMIN_AUTH_KEY, //
      verifier: new VerifierTwo(),
    });

    const appParams = {
      verifier: new MockVerifier(),
      inputs: { },
    }

    let tx = new Transaction({
      instance: lightInstance,
      payer: ADMIN_AUTH_KEYPAIR,
      shuffleEnabled: false,
      relayer
    });

    await tx.compile(txParams, appParams);
    await tx.getProof();
    await tx.getAppProof();
    await tx.sendAndConfirmTransaction();
    await updateMerkleTreeForTest(provider)
  });

});
