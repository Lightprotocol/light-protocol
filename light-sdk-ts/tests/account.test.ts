import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
let circomlibjs = require("circomlibjs");
import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt, buildBabyjub, buildEddsa } from "circomlibjs";
import { Scalar } from "ffjavascript";

import { Account } from "../src/account";
import {
  AccountError,
  AccountErrorCode,
  newNonce,
  TransactionParametersErrorCode,
} from "../src";
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
let seed32 = new Uint8Array(32).fill(1).toString();

describe("Test Account Functional", () => {
  var poseidon, eddsa, babyJub, F, k0: Account, k00: Account, kBurner: Account;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    eddsa = await buildEddsa();
    babyJub = await buildBabyjub();
    F = babyJub.F;
    k0 = new Account({ poseidon, seed: seed32 });
    k00 = new Account({ poseidon, seed: seed32 });
    kBurner = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
  });

  it("Test blake2 Domain separation", () => {
    let seed = "123";
    let seedHash = blake2b.create(b2params).update(seed).digest();
    let encSeed = seedHash + "encryption";
    let privkeySeed = seedHash + "privkey";
    let privkeyHash = blake2b.create(b2params).update(privkeySeed).digest();
    let encHash = blake2b.create(b2params).update(encSeed).digest();

    assert.notEqual(encHash, seedHash);
    assert.notEqual(privkeyHash, seedHash);
    assert.notEqual(encHash, privkeyHash);
  });

  it("Test poseidon", async () => {
    let x = new Array(30).fill(1);
    let y = new Array(30).fill(2);

    let hash = poseidon.F.toString(
      poseidon([new anchor.BN(x).toString(), new anchor.BN(y).toString()]),
    );

    x = new Array(29).fill(1);
    y = new Array(31).fill(2);
    y[30] = 1;

    const hash1 = poseidon.F.toString(
      poseidon([new anchor.BN(x).toString(), new anchor.BN(y).toString()]),
    );
    assert.notEqual(hash, hash1);
  });

  it("Test Poseidon Eddsa Keypair", async () => {
    let seed32 = new Uint8Array(32).fill(1).toString();
    let k0 = new Account({ poseidon, seed: seed32, eddsa });

    const prvKey = blake2b
      .create(b2params)
      .update(seed32 + "poseidonEddsaKeypair")
      .digest();
    const pubKey = eddsa.prv2pub(prvKey);
    k0.getEddsaPublicKey();
    if (k0.poseidonEddsaKeypair && k0.poseidonEddsaKeypair.publicKey) {
      assert.equal(
        prvKey.toString(),
        k0.poseidonEddsaKeypair.privateKey.toString(),
      );
      assert.equal(
        pubKey[0].toString(),
        k0.poseidonEddsaKeypair.publicKey[0].toString(),
      );
      assert.equal(
        pubKey[1].toString(),
        k0.poseidonEddsaKeypair.publicKey[1].toString(),
      );
    } else {
      throw new Error("k0.poseidonEddsaKeypair undefined");
    }

    const msg = "12321";
    const sigK0 = await k0.signEddsa(msg);
    assert.equal(
      sigK0.toString(),
      eddsa.packSignature(eddsa.signPoseidon(prvKey, F.e(Scalar.e(msg)))),
    );
    assert(eddsa.verifyPoseidon(msg, eddsa.unpackSignature(sigK0), pubKey));
  });
  const compareKeypairsEqual = (
    k0: Account,
    k1: Account,
    fromPrivkey: Boolean = false,
  ) => {
    assert.equal(k0.privkey.toString(), k1.privkey.toString());
    assert.equal(k0.pubkey.toString(), k1.pubkey.toString());
    assert.equal(k0.burnerSeed.toString(), k1.burnerSeed.toString());
    assert.equal(k0.aesSecret?.toString(), k1.aesSecret?.toString());
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

  it("Functional", async () => {
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
    assert.equal(
      (await k0.signEddsa("12321")).toString(),
      "212,157,228,136,102,128,200,55,198,76,182,145,197,253,21,162,44,1,96,155,169,90,154,102,119,222,224,151,18,121,71,15,96,116,148,29,69,204,94,22,119,89,152,185,128,45,25,73,227,245,247,13,19,51,95,1,86,67,111,212,63,92,213,0",
    );

    assert.equal(
      (await k0.signEddsa("12321", eddsa)).toString(),
      "212,157,228,136,102,128,200,55,198,76,182,145,197,253,21,162,44,1,96,155,169,90,154,102,119,222,224,151,18,121,71,15,96,116,148,29,69,204,94,22,119,89,152,185,128,45,25,73,227,245,247,13,19,51,95,1,86,67,111,212,63,92,213,0",
    );

    assert.equal(
      (await k0.signEddsa("12321", eddsa)).toString(),
      "212,157,228,136,102,128,200,55,198,76,182,145,197,253,21,162,44,1,96,155,169,90,154,102,119,222,224,151,18,121,71,15,96,116,148,29,69,204,94,22,119,89,152,185,128,45,25,73,227,245,247,13,19,51,95,1,86,67,111,212,63,92,213,0",
    );

    let seedDiff32 = new Uint8Array(32).fill(2).toString();
    let k1 = new Account({ poseidon, seed: seedDiff32 });
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(k0, k1);
  });

  it("Burner functional", async () => {
    // functional reference burner
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

    assert.equal(
      (await kBurner.signEddsa("12321")).toString(),
      "79,54,246,128,173,120,190,144,139,170,213,115,226,103,155,253,214,137,30,177,186,67,128,53,164,240,81,55,138,98,181,34,121,204,42,16,191,189,18,169,230,169,65,46,94,168,211,137,21,79,175,171,187,86,59,162,202,118,45,229,189,84,146,2",
    );

    assert.equal(
      (await kBurner.signEddsa("12321", eddsa)).toString(),
      "79,54,246,128,173,120,190,144,139,170,213,115,226,103,155,253,214,137,30,177,186,67,128,53,164,240,81,55,138,98,181,34,121,204,42,16,191,189,18,169,230,169,65,46,94,168,211,137,21,79,175,171,187,86,59,162,202,118,45,229,189,84,146,2",
    );

    // burners and regular keypair from the same seed are not equal
    compareKeypairsNotEqual(k0, kBurner, true);

    let kBurner2 = Account.fromBurnerSeed(poseidon, kBurner.burnerSeed);
    compareKeypairsEqual(kBurner2, kBurner);
    compareKeypairsNotEqual(k0, kBurner2, true);
  });

  it("Burner same index & keypair eq", () => {
    let kBurner0 = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
    // burners with the same index from the same seed are the equal
    compareKeypairsEqual(kBurner0, kBurner);
  });

  it("Burner diff index & keypair neq", () => {
    let kBurner0 = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
    // burners with the same index from the same seed are the equal
    compareKeypairsEqual(kBurner0, kBurner);
    let kBurner1 = Account.createBurner(poseidon, seed32, new anchor.BN("1"));
    // burners with incrementing index are not equal
    compareKeypairsNotEqual(kBurner1, kBurner0, true);
  });

  it("fromPrivkey", () => {
    if(!k0.aesSecret)
      throw new Error("Aes key is undefined");
    let k0Privkey = Account.fromPrivkey(
      poseidon,
      k0.privkey.toBuffer("be", 32),
      k0.encryptionKeypair.secretKey,
      k0.aesSecret,
    );
    compareKeypairsEqual(k0Privkey, k0, true);
  });

  it("fromPubkey", () => {
    let k0Pubkey = Account.fromPubkey(
      k0.pubkey.toBuffer("be", 32),
      k0.encryptionKeypair.publicKey,
      poseidon,
    );
    assert.equal(k0Pubkey.pubkey.toString(), k0.pubkey.toString());
    assert.notEqual(k0Pubkey.privkey, k0.privkey);
  });

  it("aes encryption", async () => {
    let message = new Uint8Array(32).fill(1);
    // never reuse nonces this is only for testing
    let nonce = newNonce().subarray(0,16)
    if (!k0.aesSecret)
      throw  new Error("aes secret undefined")

    // removed domain separation
    let cipherText1 = await Account.encryptAes(k0.aesSecret, message, nonce);
    // let cipherText2 = await Account.encryptAes(k0.aesSecret,message, nonce, "newDomain");
    let cleartext1 = await Account.decryptAes(k0.aesSecret,cipherText1);
    // let cleartext2 = await Account.decryptAes(k0.aesSecret,cipherText2, "newDomain");

    // assert.notEqual(cipherText1.toString(), cipherText2.toString());
    // assert.equal(cleartext1.toString(), cleartext2.toString());
    assert.equal(cleartext1.toString(), message.toString());
    // try to decrypt with invalid secret key
    await chai.assert.isRejected(
      Account.decryptAes(new Uint8Array(32).fill(1),cipherText1),
      Error
    )
  });
});

describe("Test Account Errors", () => {
  var poseidon, eddsa, babyJub, F, k0: Account, k00: Account, kBurner: Account;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    eddsa = await buildEddsa();
    babyJub = await buildBabyjub();
    F = babyJub.F;
    k0 = new Account({ poseidon, seed: seed32 });
    k00 = new Account({ poseidon, seed: seed32 });
    kBurner = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
  });

  it("INVALID_SEED_SIZE", async () => {
    expect(() => {
      new Account({ poseidon, seed: "123" });
    })
      .to.throw(AccountError)
      .includes({
        code: AccountErrorCode.INVALID_SEED_SIZE,
        functionName: "constructor",
      });
  });

  it("INVALID_SEED_SIZE burner", async () => {
    expect(() => {
      new Account({ poseidon, seed: "123", burner: true });
    })
      .to.throw(AccountError)
      .includes({
        code: AccountErrorCode.INVALID_SEED_SIZE,
        functionName: "constructor",
      });
  });

  it("NO_POSEIDON_HASHER_PROVIDED", async () => {
    expect(() => {
      new Account({ seed: "123" });
    })
      .to.throw(AccountError)
      .includes({
        code: TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
        functionName: "constructor",
      });
  });

  it("ENCRYPTION_PRIVATE_KEY_UNDEFINED", async () => {
    expect(() => {
      // @ts-ignore
      Account.fromPrivkey(poseidon, k0.privkey.toBuffer("be", 32));
    })
      .to.throw(AccountError)
      .includes({
        code: AccountErrorCode.ENCRYPTION_PRIVATE_KEY_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("AES_SECRET_UNDEFINED", () => {
    expect(() => {
      // @ts-ignore
      Account.fromPrivkey(
        poseidon,
        k0.privkey.toBuffer("be", 32),
        k0.encryptionKeypair.secretKey,
      );
    })
      .to.throw(AccountError)
      .includes({
        code: AccountErrorCode.AES_SECRET_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("POSEIDON_EDDSA_KEYPAIR_UNDEFINED getEddsaPublicKey", async () => {
    const account = Account.fromPubkey(
      k0.pubkey.toBuffer("be", 32),
      k0.encryptionKeypair.publicKey,
      poseidon,
    );
    await chai.assert.isRejected(
      account.getEddsaPublicKey(),
      AccountErrorCode.POSEIDON_EDDSA_KEYPAIR_UNDEFINED,
    );
  });

  it("POSEIDON_EDDSA_KEYPAIR_UNDEFINED signEddsa", async () => {
    const account = Account.fromPubkey(
      k0.pubkey.toBuffer("be", 32),
      k0.encryptionKeypair.publicKey,
      poseidon,
    );
    await chai.assert.isRejected(
      account.signEddsa("123123"),
      AccountErrorCode.POSEIDON_EDDSA_KEYPAIR_UNDEFINED,
    );
  });
});
