import {
  ADMIN_AUTH_KEYPAIR,
  confirmConfig,
  FEE_ASSET,
  Keypair,
  LightInstance,
  MINT,
  Transaction,
  TransactionParameters,
  Utxo,
  VerifierZero,
} from "../index";
import { MerkleTree, SolMerkleTree } from "../merkleTree/index";
import * as anchor from "@coral-xyz/anchor";
import { assert, expect } from "chai";
import { Connection, Keypair as SolanaKeypair } from "@solana/web3.js";
import { getProvider } from "@coral-xyz/anchor";
const circomlibjs = require("circomlibjs");

export async function functionalCircuitTest() {
  try {
    const provider = new anchor.AnchorProvider(
      await new Connection("http://127.0.0.1:8899"),
      new anchor.Wallet(SolanaKeypair.generate()),
      confirmConfig,
    );
    await anchor.setProvider(provider);
  } catch (error) {
    console.log("expected local test validator to be running");
    process.exit();
  }

  const poseidon = await circomlibjs.buildPoseidonOpt();
  let seed32 = new Uint8Array(32).fill(1).toString();
  let keypair = new Keypair({ poseidon: poseidon, seed: seed32 });
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;
  let deposit_utxo1 = new Utxo({
    poseidon: poseidon,
    assets: [FEE_ASSET, MINT],
    amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
    keypair,
  });
  let mockPubkey = SolanaKeypair.generate().publicKey;

  let lightInstance: LightInstance = {
    solMerkleTree: new SolMerkleTree({ poseidon, pubkey: mockPubkey }),
  };

  let txParams = new TransactionParameters({
    outputUtxos: [deposit_utxo1],
    merkleTreePubkey: mockPubkey,
    sender: mockPubkey,
    senderFee: mockPubkey,
    verifier: new VerifierZero(),
  });

  let tx = new Transaction({
    instance: lightInstance,
    payer: ADMIN_AUTH_KEYPAIR,
  });

  // successful proofgeneration
  await tx.compile(txParams);
  console.log(tx.proofInput);

  await tx.getProof();
  // unsuccessful proofgeneration
  try {
    tx.proofInput.inIndices[0][1][1] = "1";
    // TODO: investigate why this does not kill the proof
    tx.proofInput.inIndices[0][1][0] = "1";
    expect(await tx.getProof()).to.Throw();
    // console.log(tx.input.inIndices[0])
    // console.log(tx.input.inIndices[1])
  } catch (error) {
    assert.isTrue(error.toString().includes("CheckIndices_3 line:"));
  }
}
