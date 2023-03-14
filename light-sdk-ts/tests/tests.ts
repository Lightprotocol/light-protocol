import { assert, expect } from "chai";
var chaiAsPromised = require("chai-as-promised");
let circomlibjs = require("circomlibjs");
import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt, buildBabyjub, buildEddsa } from "circomlibjs";
import { Scalar } from "ffjavascript";

import { Account } from "../src/account";
import { Utxo } from "../src/utxo";
import {
  FEE_ASSET,
  functionalCircuitTest,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Transaction,
  UtxoError,
  UtxoErrorCode,
  TransactionError,
  TransactionErrorCode,
  ProviderErrorCode,
  Provider,
  TransactionParameters,
  VerifierZero,
  Action,
  Relayer,
} from "../src";
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };

describe("verifier_program", () => {
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  it("Test poseidon", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();

    let x = new Array(32).fill(1);
    let y = new Array(32).fill(2);

    let hash = poseidon.F.toString(
      poseidon([new anchor.BN(x).toString(), new anchor.BN(y).toString()]),
    );
    console.log(new anchor.BN(hash).toArray("le", 32));

    x = new Array(32).fill(3);
    y = new Array(32).fill(3);

    hash = poseidon.F.toString(
      poseidon([new anchor.BN(x).toString(), new anchor.BN(y).toString()]),
    );
    console.log(new anchor.BN(hash).toArray("be", 32));
  });

  it("Test Keypair Poseidon Eddsa", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let eddsa = await buildEddsa();
    const babyJub = await buildBabyjub();
    const F = babyJub.F;
    let seed32 = new Uint8Array(32).fill(1).toString();
    let k0 = new Account({ poseidon, seed: seed32, eddsa });

    const prvKey = blake2b
      .create(b2params)
      .update(seed32 + "poseidonEddsa")
      .digest();
    const pubKey = eddsa.prv2pub(prvKey);
    k0.getEddsaPublicKey();
    if (k0.poseidonEddsa && k0.poseidonEddsa.publicKey) {
      assert.equal(prvKey.toString(), k0.poseidonEddsa.privateKey.toString());
      assert.equal(
        pubKey[0].toString(),
        k0.poseidonEddsa.publicKey[0].toString(),
      );
      assert.equal(
        pubKey[1].toString(),
        k0.poseidonEddsa.publicKey[1].toString(),
      );
    } else {
      throw new Error("k0.poseidonEddsa undefined");
    }

    const msg = "12321";
    const sigK0 = await k0.signEddsa(msg);
    assert.equal(
      sigK0.toString(),
      eddsa.packSignature(eddsa.signPoseidon(prvKey, F.e(Scalar.e(msg)))),
    );
    assert(eddsa.verifyPoseidon(msg, eddsa.unpackSignature(sigK0), pubKey));
  });

  // TODO: rename to 'Test Account'
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
      expect(new Account({ poseidon, seed: "123" })).to.throw();
    } catch (e) {
      assert.isTrue(
        e.toString().includes("seed too short length less than 32"),
      );
    }

    const compareKeypairsEqual = (
      k0: Account,
      k1: Account,
      fromPrivkey: Boolean = false,
    ) => {
      assert.equal(k0.privkey.toString(), k1.privkey.toString());
      assert.equal(k0.pubkey.toString(), k1.pubkey.toString());
      assert.equal(k0.burnerSeed.toString(), k1.burnerSeed.toString());
      if (!fromPrivkey) {
        assert.equal(
          k0.encryptionKeypair.publicKey.toString(),
          k1.encryptionKeypair.publicKey.toString(),
        );
      }
    };

    const compareKeypairsNotEqual = (
      k0: Account,
      k1: Account,
      burner = false,
    ) => {
      assert.notEqual(k0.privkey.toString(), k1.privkey.toString());
      assert.notEqual(
        k0.encryptionKeypair.publicKey.toString(),
        k1.encryptionKeypair.publicKey.toString(),
      );
      assert.notEqual(k0.pubkey.toString(), k1.pubkey.toString());
      if (burner) {
        assert.notEqual(k0.burnerSeed.toString(), k1.burnerSeed.toString());
      }
    };

    let seed32 = new Uint8Array(32).fill(1).toString();
    let k0 = new Account({ poseidon, seed: seed32 });
    let k00 = new Account({ poseidon, seed: seed32 });
    // generate the same keypair from seed
    compareKeypairsEqual(k0, k00);

    // functional reference
    assert.equal(
      k0.encryptionKeypair.publicKey.toString(),
      "79,88,143,40,214,78,70,137,196,5,122,152,24,73,163,196,183,217,173,186,135,188,91,113,160,128,183,111,110,245,183,96",
    );
    assert.equal(
      k0.privkey.toString(),
      "72081772318062199533713901017818635304770734661701934546410527310990294418314",
    );
    assert.equal(
      k0.pubkey.toString(),
      "17617449169454204288593541557256537870126094878332671558512052528902373564643",
    );

    let seedDiff32 = new Uint8Array(32).fill(2).toString();
    let k1 = new Account({ poseidon, seed: seedDiff32 });
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(k0, k1);

    // functional reference burner
    let kBurner = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
    assert.equal(
      kBurner.encryptionKeypair.publicKey.toString(),
      "118,44,67,51,130,2,17,15,16,119,197,218,27,218,191,249,95,51,193,62,252,27,59,71,151,12,244,206,103,244,155,13",
    );
    assert.equal(
      kBurner.privkey.toString(),
      "81841610170886826015335465607758273107896278528010278185780510216694719969226",
    );
    assert.equal(
      kBurner.pubkey.toString(),
      "3672531747475455051184163226139092471034744667609536681047180780320195966514",
    );
    assert.equal(
      Array.from(kBurner.burnerSeed).toString(),
      "142,254,65,39,85,90,174,142,146,117,207,76,115,140,59,91,85,155,236,166,1,144,219,206,240,188,218,10,215,93,41,213",
    );

    // burners and regular keypair from the same seed are not equal
    compareKeypairsNotEqual(k0, kBurner, true);
    let kBurner0 = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
    // burners with the same index from the same seed are the equal
    compareKeypairsEqual(kBurner0, kBurner);
    let kBurner1 = Account.createBurner(poseidon, seed32, new anchor.BN("1"));
    // burners with incrementing index are not equal
    compareKeypairsNotEqual(kBurner1, kBurner0, true);

    let kBurner2 = Account.fromBurnerSeed(poseidon, kBurner.burnerSeed);
    compareKeypairsEqual(kBurner2, kBurner);
    compareKeypairsNotEqual(k0, kBurner2, true);

    // fromPrivkey
    let k0Privkey = Account.fromPrivkey(
      poseidon,
      k0.privkey.toBuffer("be", 32),
    );
    compareKeypairsEqual(k0Privkey, k0, true);

    // fromPubkey
    let k0Pubkey = Account.fromPubkey(
      k0.pubkey.toBuffer("be", 32),
      k0.encryptionKeypair.publicKey,
      poseidon,
    );
    assert.equal(k0Pubkey.pubkey.toString(), k0.pubkey.toString());
    assert.notEqual(k0Pubkey.privkey, k0.privkey);
  });
});
