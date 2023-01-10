import { ADMIN_AUTH_KEYPAIR, ENCRYPTION_KEYPAIR, FEE_ASSET, hashAndTruncateToCircuit, Keypair, MerkleTree, MINT, REGISTERED_POOL_PDA_SPL_TOKEN, Transaction, Utxo, VerifierZero } from "../index";
import * as anchor from "@coral-xyz/anchor"
import { SystemProgram } from "@solana/web3.js";
import { assert, expect } from "chai";
const circomlibjs = require("circomlibjs");

export async function functionalCircuitTest() {
    console.log("disabled following prints");
    
    console.log = () => {}
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString()
    let keypair = new Keypair({poseidon: poseidon,seed: seed32});
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    let tx = new Transaction({
      payer:                  ADMIN_AUTH_KEYPAIR,
      encryptionKeypair:      ENCRYPTION_KEYPAIR,

      // four static config fields
      merkleTree: new MerkleTree(18, poseidon),
      provider: undefined,
      lookupTable:            undefined,

      relayerRecipient:       ADMIN_AUTH_KEYPAIR.publicKey,

      verifier: new VerifierZero(),
      shuffleEnabled: false,
      poseidon: poseidon
    });

    let deposit_utxo1 = new Utxo({poseidon: poseidon, assets: [FEE_ASSET,MINT], amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],  keypair})

    let outputUtxos = [deposit_utxo1];
    console.log("outputUtxos[0].assetsCircuit[1]: ", outputUtxos[0].assetsCircuit[1]);
    
    await tx.prepareTransactionFull({
      inputUtxos: [],
      outputUtxos,
      action: "DEPOSIT",
      assetPubkeys: [new anchor.BN(0), outputUtxos[0].assetsCircuit[1]],
      relayerFee: 0,
      sender: SystemProgram.programId,
      mintPubkey: hashAndTruncateToCircuit(MINT.toBytes()),
      merkleTreeAssetPubkey:  REGISTERED_POOL_PDA_SPL_TOKEN,
      config: {in: 2, out: 2}
    });
    // successful proofgen
    await tx.getProof();

    // unsuccessful proofgen
    tx.inIndices[0][1][1]='1';
    // TODO: investigate why this does not kill the proof
    tx.inIndices[0][1][0]='1';
    try {
      expect(await tx.getProof()).to.Throw();
      // console.log(tx.input.inIndices[0])
      // console.log(tx.input.inIndices[1])
    } catch (error) {
      assert.isTrue(error.toString()
        .includes("CheckIndices_3 line:"))
    }
}