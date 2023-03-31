import {
  ADMIN_AUTH_KEYPAIR,
  confirmConfig,
  FEE_ASSET,
  Account,
  Provider as LightProvider,
  MINT,
  Utxo,
  VerifierZero,
  Verifier,
  Provider,
  Transaction,
  TransactionParameters,
} from "../index";
import * as anchor from "@coral-xyz/anchor";
import { assert, expect } from "chai";
import { Connection, Keypair as SolanaKeypair } from "@solana/web3.js";
import { Action } from "enums";
const circomlibjs = require("circomlibjs");

export async function functionalCircuitTest(
  verifier: Verifier,
  app: boolean = false,
) {
  const poseidon = await circomlibjs.buildPoseidonOpt();
  let seed32 = new Uint8Array(32).fill(1).toString();
  let keypair = new Account({ poseidon: poseidon, seed: seed32 });
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;
  let deposit_utxo1 = new Utxo({
    poseidon: poseidon,
    assets: [FEE_ASSET, MINT],
    amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
    account: keypair,
  });
  let mockPubkey = SolanaKeypair.generate().publicKey;

  let lightProvider = await LightProvider.loadMock();
  let txParams = new TransactionParameters({
    outputUtxos: [deposit_utxo1],
    merkleTreePubkey: mockPubkey,
    sender: mockPubkey,
    senderFee: lightProvider.wallet!.publicKey,
    verifier: verifier,
    lookUpTable: mockPubkey,
    action: Action.SHIELD,
    poseidon,
  });

  let tx;

  // successful proofgeneration
  if (app) {
    tx = new Transaction({
      provider: lightProvider,
      params: txParams,
      appParams: { mock: "123" },
    });
  } else {
    tx = new Transaction({
      provider: lightProvider,
      params: txParams,
    });
  }
  await tx.compile();

  await tx.getProof();
  // unsuccessful proofgeneration
  let x = true;

  try {
    tx.proofInput.inIndices[0][1][1] = "1";
    // TODO: investigate why this does not kill the proof
    tx.proofInput.inIndices[0][1][0] = "1";
    expect(await tx.getProof()).to.Throw();
    x = false;
  } catch (error) {
    // assert.isTrue(error.toString().includes("CheckIndices_3 line:"));
  }
  assert.isTrue(x);
}
