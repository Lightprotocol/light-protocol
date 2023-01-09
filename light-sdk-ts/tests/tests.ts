/* -r ts-node/register -r tsconfig-paths/register*/

import { assert, expect } from "chai";
let circomlibjs = require("circomlibjs");
import { SystemProgram, Keypair as SolanaKeypair, PublicKey } from '@solana/web3.js';
import * as anchor from "@coral-xyz/anchor";
// import nacl from "tweetnacl";
const nacl = require("tweetnacl");
// require('ts-mocha');

// import {
//   MERKLE_TREE_KEY,
//   ADMIN_AUTH_KEYPAIR,
//   AUTHORITY,
//   merkleTreeProgram,
//   verifierProgramZero,
//   verifierProgramOne,
//   MINT,
//   REGISTERED_POOL_PDA_SPL,
//   REGISTERED_POOL_PDA_SOL,
//   KEYPAIR_PRIVKEY,
//   REGISTERED_VERIFIER_PDA,
//   REGISTERED_VERIFIER_ONE_PDA,
//   PRE_INSERTED_LEAVES_INDEX,
//   REGISTERED_POOL_PDA_SPL_TOKEN,
//   AUTHORITY_ONE,
//   TOKEN_AUTHORITY,
//   MERKLE_TREE_AUTHORITY_PDA,
//   USER_TOKEN_ACCOUNT,
//   RECIPIENT_TOKEN_ACCOUNT,
//   userTokenAccount,
//   recipientTokenAccount,
//   FEE_ASSET,
//   ENCRYPTION_KEYPAIR,

// } from '../src/constants'
import { Keypair } from "../src/keypair";
import { Utxo } from "../src/utxo";
import { ADMIN_AUTH_KEYPAIR, ASSET_1_ORG, ENCRYPTION_KEYPAIR, FEE_ASSET, hashAndTruncateToCircuit, MerkleTree, MINT, REGISTERED_POOL_PDA_SPL_TOKEN, Transaction, VerifierZero } from "../src";
import { unwatchFile } from "fs";

var POSEIDON, KEYPAIR
const { blake2b } = require('@noble/hashes/blake2b');
const b2params = {dkLen: 32 };

describe("verifier_program", () => {

  it("Test Keypair", async () => {
    POSEIDON = await circomlibjs.buildPoseidonOpt();
    
    let seed = "123"
    let seedHash =
      blake2b
      .create(b2params)
      .update(seed)
      .digest()
    let encSeed = seed + "encryption";
    let encHash =
      blake2b
      .create(b2params)
      .update(encSeed)
      .digest()
    let privkeySeed = seed + "privkey";
    let privkeyHash =
      blake2b
      .create(b2params)
      .update(privkeySeed)
      .digest()

    assert.notEqual(encHash, seedHash);
    assert.notEqual(privkeyHash, seedHash);
    assert.notEqual(encHash, privkeyHash);
    try{
      expect(new Keypair({poseidon: POSEIDON,seed: "123"})).to.throw()
    } catch (e) {
      assert.isTrue(e.toString().includes("seed too short length less than 32"))
    }

    const compareKeypairsEqual = (k0: Keypair, k1: Keypair, fromPrivkey: Boolean = false) => {
      assert.equal(k0.privkey.toString(), k1.privkey.toString());
      assert.equal(k0.pubkey.toString(), k1.pubkey.toString());
      assert.equal(k0.burnerSeed.toString(), k1.burnerSeed.toString());
      if(!fromPrivkey) {
        assert.equal(k0.encryptionPublicKey.toString(), k1.encryptionPublicKey.toString());
      }
    }

    const compareKeypairsNotEqual = (k0: Keypair, k1: Keypair, burner = false) => {
      assert.notEqual(k0.privkey.toString(), k1.privkey.toString());
      assert.notEqual(k0.encryptionPublicKey.toString(), k1.encryptionPublicKey.toString());
      assert.notEqual(k0.pubkey.toString(), k1.pubkey.toString());
      if (burner) {
        assert.notEqual(k0.burnerSeed.toString(), k1.burnerSeed.toString());
      }
    }

    let seed32 = new Uint8Array(32).fill(1).toString()
    let k0 = new Keypair({poseidon: POSEIDON,seed: seed32});
    let k00 = new Keypair({poseidon: POSEIDON,seed: seed32});
    // generate the same keypair from seed
    compareKeypairsEqual(k0, k00)

    // functional reference
    assert.equal(k0.encryptionPublicKey.toString(), "79,88,143,40,214,78,70,137,196,5,122,152,24,73,163,196,183,217,173,186,135,188,91,113,160,128,183,111,110,245,183,96");
    assert.equal(k0.privkey.toString(), "72081772318062199533713901017818635304770734661701934546410527310990294418314");
    assert.equal(k0.pubkey.toString(), "17617449169454204288593541557256537870126094878332671558512052528902373564643");

    let seedDiff32 = new Uint8Array(32).fill(2).toString()
    let k1 = new Keypair({poseidon: POSEIDON, seed: seedDiff32});
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(k0, k1);

    // functional reference burner
    let kBurner = Keypair.createBurner(POSEIDON, seed32, new anchor.BN("0"));
    assert.equal(kBurner.encryptionPublicKey.toString(),"118,44,67,51,130,2,17,15,16,119,197,218,27,218,191,249,95,51,193,62,252,27,59,71,151,12,244,206,103,244,155,13");
    assert.equal(kBurner.privkey.toString(), "81841610170886826015335465607758273107896278528010278185780510216694719969226");
    assert.equal(kBurner.pubkey.toString(), "3672531747475455051184163226139092471034744667609536681047180780320195966514");
    assert.equal(Array.from(kBurner.burnerSeed).toString(), "142,254,65,39,85,90,174,142,146,117,207,76,115,140,59,91,85,155,236,166,1,144,219,206,240,188,218,10,215,93,41,213");

    // burners and regular keypair from the same seed are not equal
    compareKeypairsNotEqual(k0, kBurner, true);
    let kBurner0 = Keypair.createBurner(POSEIDON, seed32, new anchor.BN("0"));
    // burners with the same index from the same seed are the equal
    compareKeypairsEqual(kBurner0, kBurner)
    let kBurner1 = Keypair.createBurner(POSEIDON, seed32, new anchor.BN("1"));
    // burners with incrementing index are not equal
    compareKeypairsNotEqual(kBurner1, kBurner0, true);

    let kBurner2 = Keypair.fromBurnerSeed(POSEIDON, kBurner.burnerSeed);
    compareKeypairsEqual(kBurner2, kBurner)
    compareKeypairsNotEqual(k0, kBurner2, true);


    // fromPrivkey
    let k0Privkey = Keypair.fromPrivkey(POSEIDON, k0.privkey.toBuffer('be', 32));
    compareKeypairsEqual(k0Privkey, k0, true);

    // fromPubkey
    let k0Pubkey = Keypair.fromPubkey(POSEIDON, k0.pubkey.toBuffer('be', 32));
    assert.equal(k0Pubkey.pubkey.toString(), k0.pubkey.toString());
    assert.notEqual(k0Pubkey.privkey, k0.privkey);
  })

  it("Test Utxo encryption", async () => {
    POSEIDON = await circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString()
    let keypair = new Keypair({poseidon: POSEIDON,seed: seed32});
    let amountFee = "1";
    let amountToken = "2";
    let assetPubkey = MINT;
    let assets = [SystemProgram.programId, assetPubkey]
    let amounts = [new anchor.BN(amountFee),new anchor.BN(amountToken)];
    let utxo0 = new Utxo(
      {
        poseidon:   POSEIDON,
        assets,
        amounts,
        keypair
      }
    )
    
    console.log(utxo0);

    // functional
    assert.equal(utxo0.amounts[0].toString(), amountFee);
    assert.equal(utxo0.amounts[1].toString(), amountToken);
    assert.equal(utxo0.assets[0].toBase58(), SystemProgram.programId.toBase58());
    assert.equal(utxo0.assets[1].toBase58(), assetPubkey.toBase58());
    assert.equal(utxo0.assetsCircuit[0].toString(), "0");
    assert.equal(utxo0.assetsCircuit[1].toString(), hashAndTruncateToCircuit(assetPubkey.toBytes()).toString());
    assert.equal(utxo0.instructionType.toString(), "0");
    assert.equal(utxo0.poolType.toString(), "0");
    assert.equal(utxo0.verifierAddress.toString(), "0");
    assert.equal(utxo0.verifierAddressCircuit.toString(), "0");

    // toBytes

    // fromBytes
    
    // encrypt

    // decrypt

    // getNullifier when no privkey
    
    

    // console.log(utxo0);
    
    // console.log(FEE_ASSET);
    


    
    // let k = new Keypair(POSEIDON)
    /*
    let deposit_utxo1 = new Utxo({
    poseidon:POSEIDON,
    assets: [FEE_ASSET,MINT],
    amounts: [new anchor.BN(1),new anchor.BN(1)],
    keypair: KEYPAIR
    })
    
    deposit_utxo1.index = 0;
    let preCommitHash = deposit_utxo1.getCommitment();
    let preNullifier = deposit_utxo1.getNullifier();

    let nonce = nacl.randomBytes(24);
    let encUtxo = deposit_utxo1.encrypt(nonce, ENCRYPTION_KEYPAIR, ENCRYPTION_KEYPAIR);
    console.log(encUtxo);
    let decUtxo = Utxo.decrypt(
    encUtxo,
    nonce,
    ENCRYPTION_KEYPAIR.PublicKey,
    ENCRYPTION_KEYPAIR,
    KEYPAIR,
    [FEE_ASSET,MINT],
    POSEIDON,
    0
    )[1];

    // console.log(decUtxo);

    assert(preCommitHash == decUtxo.getCommitment(), "commitment doesnt match")
    assert(preNullifier == decUtxo.getNullifier(), "nullifier doesnt match")
    */

  })

  // test functional circuit
  it("Test functional circuit", async () => {
    console.log("disabled following prints");
    
    console.log = () => {}
    POSEIDON = await circomlibjs.buildPoseidonOpt();
    let seed32 = new Uint8Array(32).fill(1).toString()
    let keypair = new Keypair({poseidon: POSEIDON,seed: seed32});
    let depositAmount = 20_000;
    let depositFeeAmount = 10_000;
    let tx = new Transaction({
      payer:                  ADMIN_AUTH_KEYPAIR,
      encryptionKeypair:      ENCRYPTION_KEYPAIR,

      // four static config fields
      merkleTree: new MerkleTree(18, POSEIDON),
      provider: undefined,
      lookupTable:            undefined,

      relayerRecipient:       ADMIN_AUTH_KEYPAIR.publicKey,

      verifier: new VerifierZero(),
      shuffleEnabled: false,
      poseidon: POSEIDON
    });

    let deposit_utxo1 = new Utxo({poseidon: POSEIDON, assets: [FEE_ASSET,MINT], amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],  keypair})

    let outputUtxos = [deposit_utxo1];
    console.log("outputUtxos[0].assetsCircuit[1]: ", outputUtxos[0].assetsCircuit[1]);
    
    await tx.prepareTransactionFull({
      inputUtxos: [],
      outputUtxos,
      action: "DEPOSIT",
      assetPubkeys: [FEE_ASSET, outputUtxos[0].assetsCircuit[1]],
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
    
    // provider
    // getRootIndex
    // checkBalances

  })
})