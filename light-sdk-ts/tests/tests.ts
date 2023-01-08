  /* -r ts-node/register -r tsconfig-paths/register*/

import { assert } from "chai";
let circomlibjs = require("circomlibjs");
import { SystemProgram, Keypair as SolanaKeypair } from '@solana/web3.js';
import * as anchor from "@coral-xyz/anchor";
import nacl from "tweetnacl";
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

      let k0 = new Keypair(POSEIDON, "12321");
      let k00 = new Keypair(POSEIDON, "12321");

      let k1 = new Keypair(POSEIDON, "12322");
      // generate the same keypair from seed
      assert.equal(k0.privkey.toString(), k00.privkey.toString());
      assert.equal(k0.encryptionKey.toString(), k00.encryptionKey.toString());
      assert.equal(k0.pubkey.toString(), k00.pubkey.toString());
      // generate different keypairs from different seeds
      assert.notEqual(k0.privkey.toString(), k1.privkey.toString());
      assert.notEqual(k0.encryptionKey.toString(), k1.encryptionKey.toString());
      assert.notEqual(k0.pubkey.toString(), k1.pubkey.toString());

      let k2 = new Keypair(POSEIDON, "12321", new anchor.BN("0"));

      assert.notEqual(k0.privkey.toString(), k2.privkey.toString());
      assert.notEqual(k0.encryptionKey.toString(), k2.encryptionKey.toString());
      assert.notEqual(k0.pubkey.toString(), k2.pubkey.toString());

    })

    it.skip("Test Utxo encryption", async () => {
        POSEIDON = await circomlibjs.buildPoseidonOpt();
        let keypair = new Keypair(POSEIDON, "12321");
        let assetPubkey = SolanaKeypair.generate().publicKey;
        let utxo0 = new Utxo(
          {
            poseidon:POSEIDON,
            assets: [SystemProgram.programId, assetPubkey],
            amounts: [new anchor.BN(1),new anchor.BN(2)],
            keypair
          }
          )
        assert.equal(utxo0.amounts[0].toString(), "1");
        assert.equal(utxo0.amounts[1].toString(), "2");
        assert.equal(utxo0.assets[0].toBase58(), SystemProgram.programId.toBase58());
        assert.equal(utxo0.assets[1].toBase58(), assetPubkey.toBase58());

        console.log(utxo0);
        
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
})