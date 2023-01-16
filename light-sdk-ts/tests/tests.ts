/* -r ts-node/register -r tsconfig-paths/register*/

import { assert, expect } from "chai";
let circomlibjs = require("circomlibjs");
import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
} from "@solana/web3.js";
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
import { functionalCircuitTest, hashAndTruncateToCircuit, MINT } from "../src";

var tx;
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };

describe("verifier_program", () => {
  it.skip("Test poseidon", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();

    let x = new Array(32).fill(1);
    let y = new Array(32).fill(2);

    let hash = poseidon.F.toString(
      poseidon([new anchor.BN(x).toString(), new anchor.BN(y).toString()])
    );
    console.log(new anchor.BN(hash).toArray("le", 32));

    x = new Array(32).fill(3);
    y = new Array(32).fill(3);

    hash = poseidon.F.toString(
      poseidon([new anchor.BN(x).toString(), new anchor.BN(y).toString()])
    );
    console.log(new anchor.BN(hash).toArray("be", 32));
  });
  it("Test Keypair", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();

    let seed = "123";
    let seedHash = blake2b.create(b2params).update(seed).digest();
    let encSeed = seed + "encryption";
    let encHash = blake2b.create(b2params).update(encSeed).digest();
    let privkeySeed = seed + "privkey";
    let privkeyHash = blake2b.create(b2params).update(privkeySeed).digest();

    assert.notEqual(encHash, seedHash);
    assert.notEqual(privkeyHash, seedHash);
    assert.notEqual(encHash, privkeyHash);
    try {
      expect(new Keypair({ poseidon, seed: "123" })).to.throw();
    } catch (e) {
      assert.isTrue(
        e.toString().includes("seed too short length less than 32")
      );
    }

    const compareKeypairsEqual = (
      k0: Keypair,
      k1: Keypair,
      fromPrivkey: Boolean = false
    ) => {
      assert.equal(k0.privkey.toString(), k1.privkey.toString());
      assert.equal(k0.pubkey.toString(), k1.pubkey.toString());
      assert.equal(k0.burnerSeed.toString(), k1.burnerSeed.toString());
      if (!fromPrivkey) {
        assert.equal(
          k0.encryptionPublicKey.toString(),
          k1.encryptionPublicKey.toString()
        );
      }
    };

    const compareKeypairsNotEqual = (
      k0: Keypair,
      k1: Keypair,
      burner = false
    ) => {
      assert.notEqual(k0.privkey.toString(), k1.privkey.toString());
      assert.notEqual(
        k0.encryptionPublicKey.toString(),
        k1.encryptionPublicKey.toString()
      );
      assert.notEqual(k0.pubkey.toString(), k1.pubkey.toString());
      if (burner) {
        assert.notEqual(k0.burnerSeed.toString(), k1.burnerSeed.toString());
      }
    };

    let seed32 = new Uint8Array(32).fill(1).toString();
    let k0 = new Keypair({ poseidon, seed: seed32 });
    let k00 = new Keypair({ poseidon, seed: seed32 });
    // generate the same keypair from seed
    compareKeypairsEqual(k0, k00);

    // functional reference
    assert.equal(
      k0.encryptionPublicKey.toString(),
      "79,88,143,40,214,78,70,137,196,5,122,152,24,73,163,196,183,217,173,186,135,188,91,113,160,128,183,111,110,245,183,96"
    );
    assert.equal(
      k0.privkey.toString(),
      "72081772318062199533713901017818635304770734661701934546410527310990294418314"
    );
    assert.equal(
      k0.pubkey.toString(),
      "17617449169454204288593541557256537870126094878332671558512052528902373564643"
    );

    let seedDiff32 = new Uint8Array(32).fill(2).toString();
    let k1 = new Keypair({ poseidon, seed: seedDiff32 });
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(k0, k1);

    // functional reference burner
    let kBurner = Keypair.createBurner(poseidon, seed32, new anchor.BN("0"));
    assert.equal(
      kBurner.encryptionPublicKey.toString(),
      "118,44,67,51,130,2,17,15,16,119,197,218,27,218,191,249,95,51,193,62,252,27,59,71,151,12,244,206,103,244,155,13"
    );
    assert.equal(
      kBurner.privkey.toString(),
      "81841610170886826015335465607758273107896278528010278185780510216694719969226"
    );
    assert.equal(
      kBurner.pubkey.toString(),
      "3672531747475455051184163226139092471034744667609536681047180780320195966514"
    );
    assert.equal(
      Array.from(kBurner.burnerSeed).toString(),
      "142,254,65,39,85,90,174,142,146,117,207,76,115,140,59,91,85,155,236,166,1,144,219,206,240,188,218,10,215,93,41,213"
    );

    // burners and regular keypair from the same seed are not equal
    compareKeypairsNotEqual(k0, kBurner, true);
    let kBurner0 = Keypair.createBurner(poseidon, seed32, new anchor.BN("0"));
    // burners with the same index from the same seed are the equal
    compareKeypairsEqual(kBurner0, kBurner);
    let kBurner1 = Keypair.createBurner(poseidon, seed32, new anchor.BN("1"));
    // burners with incrementing index are not equal
    compareKeypairsNotEqual(kBurner1, kBurner0, true);

    let kBurner2 = Keypair.fromBurnerSeed(poseidon, kBurner.burnerSeed);
    compareKeypairsEqual(kBurner2, kBurner);
    compareKeypairsNotEqual(k0, kBurner2, true);

    // fromPrivkey
    let k0Privkey = Keypair.fromPrivkey(
      poseidon,
      k0.privkey.toBuffer("be", 32)
    );
    compareKeypairsEqual(k0Privkey, k0, true);

    // fromPubkey
    let k0Pubkey = Keypair.fromPubkey(poseidon, k0.pubkey.toBuffer("be", 32));
    assert.equal(k0Pubkey.pubkey.toString(), k0.pubkey.toString());
    assert.notEqual(k0Pubkey.privkey, k0.privkey);
  });

  it("Test Utxo encryption", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    const amountFee = "1";
    const amountToken = "2";
    const assetPubkey = MINT;
    const seed32 = new Uint8Array(32).fill(1).toString();
    let inputs = {
      keypair: new Keypair({ poseidon, seed: seed32 }),
      amountFee,
      amountToken,
      assetPubkey,
      assets: [SystemProgram.programId, assetPubkey],
      amounts: [new anchor.BN(amountFee), new anchor.BN(amountToken)],
      blinding: new anchor.BN(new Uint8Array(31).fill(2)),
    };

    let utxo0 = new Utxo({
      poseidon,
      assets: inputs.assets,
      amounts: inputs.amounts,
      keypair: inputs.keypair,
      blinding: inputs.blinding,
    });
    // functional
    assert.equal(utxo0.amounts[0].toString(), amountFee);
    assert.equal(utxo0.amounts[1].toString(), amountToken);
    assert.equal(
      utxo0.assets[0].toBase58(),
      SystemProgram.programId.toBase58()
    );
    assert.equal(utxo0.assets[1].toBase58(), assetPubkey.toBase58());
    assert.equal(utxo0.assetsCircuit[0].toString(), hashAndTruncateToCircuit(SystemProgram.programId.toBytes()));
    assert.equal(
      utxo0.assetsCircuit[1].toString(),
      hashAndTruncateToCircuit(assetPubkey.toBytes()).toString()
    );
    assert.equal(utxo0.instructionType.toString(), "0");
    assert.equal(utxo0.poolType.toString(), "0");
    assert.equal(utxo0.verifierAddress.toString(), "0");
    assert.equal(utxo0.verifierAddressCircuit.toString(), "0");
    // assert.equal(
    //   utxo0.getCommitment()?.toString(),
    //   "7790797031264776843808539823930722966306182688678265114219628300976218020196"
    // );
    // assert.equal(
    //   utxo0.getNullifier()?.toString(),
    //   "12550499222412009082001099158167607283963476261823222797328102845322566964367"
    // );

    const utxoEqual = (utxo0: Utxo, utxo1: Utxo) => {
      assert.equal(utxo0.amounts[0].toString(), utxo1.amounts[0].toString());
      assert.equal(utxo0.amounts[1].toString(), utxo1.amounts[1].toString());
      assert.equal(utxo0.assets[0].toBase58(), utxo1.assets[0].toBase58());
      assert.equal(utxo0.assets[1].toBase58(), utxo1.assets[1].toBase58());
      assert.equal(
        utxo0.assetsCircuit[0].toString(),
        utxo1.assetsCircuit[0].toString()
      );
      assert.equal(
        utxo0.assetsCircuit[1].toString(),
        utxo1.assetsCircuit[1].toString()
      );
      assert.equal(
        utxo0.instructionType.toString(),
        utxo1.instructionType.toString()
      );
      assert.equal(utxo0.poolType.toString(), utxo1.poolType.toString());
      assert.equal(
        utxo0.verifierAddress.toString(),
        utxo1.verifierAddress.toString()
      );
      assert.equal(
        utxo0.verifierAddressCircuit.toString(),
        utxo1.verifierAddressCircuit.toString()
      );
      assert.equal(
        utxo0.getCommitment()?.toString(),
        utxo1.getCommitment()?.toString(),
      );
      assert.equal(
        utxo0.getNullifier()?.toString(),
        utxo1.getNullifier()?.toString(),
      );
    };

    // toBytes
    const bytes = utxo0.toBytes();
    // fromBytes
    const utxo1 = Utxo.fromBytes({ poseidon, keypair: inputs.keypair, bytes });
    utxoEqual(utxo0, utxo1);
    // encrypt
    const encBytes = utxo1.encrypt();

    // decrypt
    const utxo3 = Utxo.decrypt({ poseidon, encBytes, keypair: inputs.keypair });
    if (utxo3) {
      utxoEqual(utxo0, utxo3);
    } else {
      throw "decrypt failed";
    }

    // try basic tests for rnd empty utxo
    const utxo4 = new Utxo({ poseidon });
    // toBytes
    const bytes4 = utxo4.toBytes();
    // fromBytes
    const utxo40 = Utxo.fromBytes({
      poseidon,
      keypair: utxo4.keypair,
      bytes: bytes4,
    });
    utxoEqual(utxo4, utxo40);
    // encrypt
    const encBytes4 = utxo4.encrypt();
    const utxo41 = Utxo.decrypt({
      poseidon,
      encBytes: encBytes4,
      keypair: utxo4.keypair,
    });
    if (utxo41) {
      utxoEqual(utxo4, utxo41);
    } else {
      throw "decrypt failed";
    }
    console.log(new Utxo({poseidon}));

    // getNullifier when no privkey
  });

  // test functional circuit
  it("Test functional circuit", async () => {
    
    await functionalCircuitTest();
  });

  it("assign Accounts", async () => {});
});
