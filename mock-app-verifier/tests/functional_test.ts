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
  VerifierTwo
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
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local("http://127.0.0.1:8899", {
    preflightCommitment: "confirmed",
    commitment: "confirmed",
  });

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


  it("Test Deposit MockVerifier cpi VerifierTwo", async () => {
    const poseidon = await buildPoseidonOpt();

    let lightInstance: LightInstance = {
      solMerkleTree: new SolMerkleTree({ poseidon, pubkey: MERKLE_TREE_KEY }),
      lookUpTable: LOOK_UP_TABLE,
      provider
    };

    const outputUtxo = new Utxo({
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
    await tx.getProof();
    await tx.getAppProof();
    await tx.sendAndConfirmTransaction();
    // await updateMerkleTreeForTest(provider)
  });

});
