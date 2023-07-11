import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
const circomlibjs = require("circomlibjs");
const { buildBabyjub, buildEddsa } = circomlibjs;
const ffjavascript = require("ffjavascript");
const { Scalar } = ffjavascript;

import {
  Account,
  AccountError,
  AccountErrorCode,
  newNonce,
  TransactionParametersErrorCode,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
let seed32 = bs58.encode(new Uint8Array(32).fill(1));

describe("Test Account Functional", () => {
  let poseidon: any,
    eddsa: any,
    babyJub,
    F: any,
    k0: Account,
    k00: Account,
    kBurner: Account;
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
    let seed = bs58.encode([1, 2, 3]);
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
    let seed32 = bs58.encode(new Uint8Array(32).fill(1));
    let k0 = new Account({ poseidon, seed: seed32, eddsa });

    const prvKey = blake2b
      .create(b2params)
      .update(seed32 + "poseidonEddsaKeypair")
      .digest();
    const pubKey = eddsa.prv2pub(prvKey);
    await k0.getEddsaPublicKey();
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
    assert.equal(bs58.encode(k0.aesSecret!), bs58.encode(k1.aesSecret!));
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

  const compareAccountToReference = async (
    account: Account,
    reference: any,
  ) => {
    assert.equal(
      account.encryptionKeypair.publicKey.toString(),
      reference.encryptionPublicKey.toString(),
    );
    assert.equal(account.privkey.toString(), reference.privkey.toString());
    assert.equal(account.pubkey.toString(), reference.pubkey.toString());
    if (reference.burnerSeed) {
      assert.equal(
        Array.from(kBurner.burnerSeed).toString(),
        reference.burnerSeed.toString(),
      );
    }
    assert.equal(
      (await account.signEddsa("12321")).toString(),
      reference.eddsaSignature.toString(),
    );
  };

  it("Functional", async () => {
    // generate the same keypair from seed
    compareKeypairsEqual(k0, k00);
    let referenceAccount = {
      encryptionPublicKey:
        "246,239,160,64,108,202,122,119,186,218,229,31,22,26,16,217,91,100,166,215,150,23,31,160,171,11,70,146,121,162,63,118",
      privkey:
        "8005258175950153822746760972612266673018285206748118268998514552503031523041",
      pubkey:
        "6377640866559980556624371737408417701494249873246144458744315242624363752533",
      eddsaSignature:
        "49,171,181,231,94,94,233,87,62,92,132,207,160,18,252,199,169,46,131,38,9,250,202,156,232,7,147,10,62,115,216,21,224,99,163,86,218,224,115,91,107,158,231,171,120,83,79,35,221,119,92,43,69,148,166,215,39,96,194,102,65,19,238,1",
    };
    await compareAccountToReference(k0, referenceAccount);

    let seedDiff32 = bs58.encode(new Uint8Array(32).fill(2));
    let k1 = new Account({ poseidon, seed: seedDiff32 });
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(k0, k1);
  });

  it("Burner functional", async () => {
    let referenceAccount = {
      encryptionPublicKey:
        "16,138,150,240,149,102,160,39,50,184,20,203,200,49,139,7,85,228,125,46,203,5,120,152,151,35,30,68,120,245,39,57",
      privkey:
        "5505067515222742133337966884584633324908181750622530156812582813220567498363",
      pubkey:
        "3373572053317352269516743219507441053963774784739492817596773344511570546301",
      burnerSeed:
        "21,73,66,60,60,94,31,45,240,18,81,195,45,57,152,4,115,85,189,103,253,170,190,192,190,13,46,155,92,44,145,46",
      eddsaSignature:
        "43,114,239,133,220,59,32,233,39,134,131,226,64,196,102,141,235,195,197,43,213,133,176,199,208,176,254,49,72,83,81,152,148,24,18,17,222,198,197,197,248,112,220,94,108,62,185,35,130,216,88,82,19,84,210,16,51,3,213,86,77,210,74,0",
    };
    await compareAccountToReference(kBurner, referenceAccount);

    // burners and regular keypair from the same seed are not equal
    compareKeypairsNotEqual(k0, kBurner, true);

    let kBurner2 = Account.fromBurnerSeed(
      poseidon,
      bs58.encode(kBurner.burnerSeed),
    );
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
    if (!k0.aesSecret) throw new Error("Aes key is undefined");
    let { privateKey, aesSecret, encryptionPrivateKey } = k0.getPrivateKeys();
    let k0Privkey = Account.fromPrivkey(
      poseidon,
      privateKey,
      encryptionPrivateKey,
      aesSecret,
    );
    compareKeypairsEqual(k0Privkey, k0, true);
  });

  it("fromPubkey", () => {
    let pubKey = k0.getPublicKey();
    let k0Pubkey = Account.fromPubkey(pubKey, poseidon);
    assert.equal(k0Pubkey.pubkey.toString(), k0.pubkey.toString());
    assert.notEqual(k0Pubkey.privkey, k0.privkey);
  });

  it("aes encryption", async () => {
    let message = new Uint8Array(32).fill(1);
    // never reuse nonces this is only for testing
    let nonce = newNonce().subarray(0, 16);
    if (!k0.aesSecret) throw new Error("aes secret undefined");

    let cipherText1 = await Account.encryptAes(k0.aesSecret, message, nonce);
    let cleartext1 = await Account.decryptAes(k0.aesSecret, cipherText1);

    assert.equal(cleartext1.toString(), message.toString());
    assert.notEqual(
      new Uint8Array(32).fill(1).toString(),
      k0.aesSecret.toString(),
    );

    // try to decrypt with invalid secret key
    // added try catch because in some cases it doesn't decrypt but doesn't throw an error either
    //TODO: revisit this and possibly switch aes library
    try {
      await chai.assert.isRejected(
        Account.decryptAes(new Uint8Array(32).fill(1), cipherText1),
        Error,
      );
    } catch (error) {
      const msg = Account.decryptAes(new Uint8Array(32).fill(1), cipherText1);
      assert.notEqual(msg.toString(), message.toString());
    }
  });
});

describe("Test Account Errors", () => {
  let poseidon: any,
    eddsa,
    babyJub,
    F,
    k0: Account,
    k00: Account,
    kBurner: Account;
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
      new Account({ poseidon, seed: bs58.encode([1, 2, 3]) });
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
      Account.fromPrivkey(poseidon, bs58.encode(k0.privkey.toBuffer("be", 32)));
    })
      .to.throw(AccountError)
      .includes({
        code: AccountErrorCode.ENCRYPTION_PRIVATE_KEY_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("AES_SECRET_UNDEFINED", () => {
    let { privateKey, encryptionPrivateKey } = k0.getPrivateKeys();

    expect(() => {
      // @ts-ignore
      Account.fromPrivkey(poseidon, privateKey, encryptionPrivateKey);
    })
      .to.throw(AccountError)
      .includes({
        code: AccountErrorCode.AES_SECRET_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("POSEIDON_EDDSA_KEYPAIR_UNDEFINED getEddsaPublicKey", async () => {
    let pubKey = k0.getPublicKey();

    const account = Account.fromPubkey(pubKey, poseidon);
    await chai.assert.isRejected(
      account.getEddsaPublicKey(),
      AccountErrorCode.POSEIDON_EDDSA_KEYPAIR_UNDEFINED,
    );
  });

  it("POSEIDON_EDDSA_KEYPAIR_UNDEFINED signEddsa", async () => {
    let pubKey = k0.getPublicKey();

    const account = Account.fromPubkey(pubKey, poseidon);
    await chai.assert.isRejected(
      account.signEddsa("123123"),
      AccountErrorCode.POSEIDON_EDDSA_KEYPAIR_UNDEFINED,
    );
  });
});
