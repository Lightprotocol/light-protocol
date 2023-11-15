import { assert, expect } from "chai";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import {
  Account,
  AccountError,
  AccountErrorCode,
  ADMIN_AUTH_KEYPAIR,
  newNonce,
  useWallet,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Poseidon, blake } from "@lightprotocol/account.rs";

const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
const circomlibjs = require("circomlibjs");
const { buildBabyjub, buildEddsa } = circomlibjs;
const ffjavascript = require("ffjavascript");
const { Scalar } = ffjavascript;
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const seed32 = (): string => {
  return bs58.encode(new Uint8Array(32).fill(1));
};

const keypairReferenceAccount = {
  encryptionPublicKey:
    "187,15,119,127,223,162,69,232,129,87,132,195,89,178,128,174,220,77,191,34,63,115,138,98,193,57,4,92,247,18,190,114",
  privkey:
    "7314374631704302594235695652925685842509708564100145210880269088513605645300",
  pubkey:
    "6391168142226478154718281169178137802178553836996014555114884736358424922672",
  eddsaSignature:
    "149,4,55,200,119,181,112,89,28,114,19,62,250,125,9,166,167,0,255,21,231,177,123,126,100,125,212,10,93,27,186,172,107,200,130,11,182,98,146,73,73,248,205,73,73,217,201,196,85,249,115,198,152,225,175,160,254,131,131,146,148,73,211,1",
};

describe("Test Account Functional", () => {
  let poseidon: Poseidon,
    eddsa: any,
    babyJub,
    F: any,
    k0: Account,
    k00: Account,
    kBurner: Account;

  before(async () => {
    poseidon = await Poseidon.getInstance();
    eddsa = await buildEddsa();
    babyJub = await buildBabyjub();
    F = babyJub.F;
    k0 = new Account({ poseidon, seed: seed32() });
    k00 = new Account({ poseidon, seed: seed32() });
    kBurner = Account.createBurner(poseidon, seed32(), new BN("0"));
  });

  it("Test blake2 Domain separation", () => {
    const seed = bs58.encode([1, 2, 3]);
    const seedHash = blake(seed, Account.hashLength);
    const encSeed = seedHash + "encryption";
    const privkeySeed = seedHash + "privkey";
    const privkeyHash = blake(privkeySeed, Account.hashLength);
    const encHash = blake(encSeed, Account.hashLength);

    assert.notEqual(encHash, seedHash);
    assert.notEqual(privkeyHash, seedHash);
    assert.notEqual(encHash, privkeyHash);
  });

  it("Test poseidon", async () => {
    let x = new Array(30).fill(1);
    let y = new Array(30).fill(2);
    const poseidon = await Poseidon.getInstance();
    const hash = poseidon.string(
      poseidon.hash([new BN(x).toString(), new BN(y).toString()]),
    );

    x = new Array(29).fill(1);
    y = new Array(31).fill(2);
    y[30] = 1;

    const hash1 = poseidon.string(
      poseidon.hash([new BN(x).toString(), new BN(y).toString()]),
    );
    assert.notEqual(hash, hash1);
  });

  it("Test wasm poseidon", async () => {
    const poseidon = await Poseidon.getInstance();
    let x = new Array(30).fill(1);
    let y = new Array(30).fill(2);

    const hash = poseidon.hash([new BN(x).toString(), new BN(y).toString()]);
    x = new Array(29).fill(1);
    y = new Array(31).fill(2);
    y[30] = 1;

    const hash1 = poseidon.hash([new BN(x).toString(), new BN(y).toString()]);
    assert.notEqual(hash, hash1);
  });

  it("Test Poseidon Eddsa Keypair", async () => {
    const k0 = new Account({ poseidon, seed: seed32(), eddsa });

    const prvKey = blake(
      seed32() + "poseidonEddsaKeypair",
      Account.hashLength,
    );
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
    fromPrivkey: boolean = false,
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

  it("Constructor & from seed Functional", async () => {
    // generate the same keypair from seed
    compareKeypairsEqual(k0, k00);
    const referenceAccount = {
      encryptionPublicKey:
        "246,239,160,64,108,202,122,119,186,218,229,31,22,26,16,217,91,100,166,215,150,23,31,160,171,11,70,146,121,162,63,118",
      privkey:
        "17660568269376948254360594374708563282178836364116256831458469636153315805952",
      pubkey:
        "8603563756329284155374037240612788771833697010028509322943197012915150315482",
      eddsaSignature:
        "49,171,181,231,94,94,233,87,62,92,132,207,160,18,252,199,169,46,131,38,9,250,202,156,232,7,147,10,62,115,216,21,224,99,163,86,218,224,115,91,107,158,231,171,120,83,79,35,221,119,92,43,69,148,166,215,39,96,194,102,65,19,238,1",
    };
    await compareAccountToReference(k0, referenceAccount);

    const seedDiff32 = bs58.encode(new Uint8Array(32).fill(2));
    const k1 = new Account({ poseidon, seed: seedDiff32 });
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(k0, k1);

    const k2 = Account.createFromSeed(poseidon, seed32());
    compareKeypairsEqual(k0, k2);
  });

  it("createFromSolanaKeypair Functional", async () => {
    const solanaKeypairAccount = Account.createFromSolanaKeypair(
      poseidon,
      ADMIN_AUTH_KEYPAIR,
      eddsa,
    );
    await compareAccountToReference(
      solanaKeypairAccount,
      keypairReferenceAccount,
    );

    const seedDiff32 = bs58.encode(new Uint8Array(32).fill(2));
    const k1 = new Account({ poseidon, seed: seedDiff32 });
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(solanaKeypairAccount, k1);
  });

  it("createFromBrowserWallet Functional", async () => {
    const wallet = useWallet(ADMIN_AUTH_KEYPAIR);

    const solanaWalletAccount = await Account.createFromBrowserWallet(
      poseidon,
      wallet,
      eddsa,
    );
    await compareAccountToReference(
      solanaWalletAccount,
      keypairReferenceAccount,
    );

    const seedDiff32 = bs58.encode(new Uint8Array(32).fill(2));
    const k1 = new Account({ poseidon, seed: seedDiff32 });
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(solanaWalletAccount, k1);
  });

  it("Burner functional", async () => {
    const referenceAccount = {
      encryptionPublicKey:
        "16,138,150,240,149,102,160,39,50,184,20,203,200,49,139,7,85,228,125,46,203,5,120,152,151,35,30,68,120,245,39,57",
      privkey:
        "1216505848398139118357877453525011331850341160722374734246634497615230274573",
      pubkey:
        "16628940406069543878444471255790949971721743937795035096532685288799757413966",
      burnerSeed:
        "21,73,66,60,60,94,31,45,240,18,81,195,45,57,152,4,115,85,189,103,253,170,190,192,190,13,46,155,92,44,145,46",
      eddsaSignature:
        "43,114,239,133,220,59,32,233,39,134,131,226,64,196,102,141,235,195,197,43,213,133,176,199,208,176,254,49,72,83,81,152,148,24,18,17,222,198,197,197,248,112,220,94,108,62,185,35,130,216,88,82,19,84,210,16,51,3,213,86,77,210,74,0",
    };
    await compareAccountToReference(kBurner, referenceAccount);

    // burners and regular keypair from the same seed are not equal
    compareKeypairsNotEqual(k0, kBurner, true);

    const kBurner2 = Account.fromBurnerSeed(
      poseidon,
      bs58.encode(kBurner.burnerSeed),
    );
    compareKeypairsEqual(kBurner2, kBurner);
    compareKeypairsNotEqual(k0, kBurner2, true);
  });

  it("Burner same index & keypair eq", () => {
    const kBurner0 = Account.createBurner(poseidon, seed32(), new BN("0"));
    // burners with the same index from the same seed are the equal
    compareKeypairsEqual(kBurner0, kBurner);
  });

  it("Burner diff index & keypair neq", () => {
    const kBurner0 = Account.createBurner(poseidon, seed32(), new BN("0"));
    // burners with the same index from the same seed are the equal
    compareKeypairsEqual(kBurner0, kBurner);
    const kBurner1 = Account.createBurner(poseidon, seed32(), new BN("1"));
    // burners with incrementing index are not equal
    compareKeypairsNotEqual(kBurner1, kBurner0, true);
  });

  it("fromPrivkey", () => {
    if (!k0.aesSecret) throw new Error("Aes key is undefined");
    const { privateKey, aesSecret, encryptionPrivateKey } = k0.getPrivateKeys();
    const k0Privkey = Account.fromPrivkey(
      poseidon,
      privateKey,
      encryptionPrivateKey,
      aesSecret,
    );
    compareKeypairsEqual(k0Privkey, k0, true);
  });

  it("fromPubkey", () => {
    const pubKey = k0.getPublicKey();
    const k0Pubkey = Account.fromPubkey(pubKey, poseidon);
    assert.equal(k0Pubkey.pubkey.toString(), k0.pubkey.toString());
    assert.notEqual(k0Pubkey.privkey, k0.privkey);
  });

  it("aes encryption", async () => {
    const message = new Uint8Array(32).fill(1);
    // never reuse nonces this is only for testing
    const nonce = newNonce().subarray(0, 16);
    if (!k0.aesSecret) throw new Error("aes secret undefined");

    const cipherText1 = await k0.encryptAes(message, nonce);
    const cleartext1 = await k0.decryptAes(cipherText1);

    assert.equal(cleartext1.value!.toString(), message.toString());
    assert.notEqual(
      new Uint8Array(32).fill(1).toString(),
      k0.aesSecret.toString(),
    );

    // try to decrypt with invalid secretkey
    // added try catch because in some cases it doesn't decrypt but doesn't throw an error either
    //TODO: revisit this and possibly switch aes library
    try {
      await chai.assert.isRejected(kBurner.decryptAes(cipherText1), Error);
    } catch (error) {
      const msg = k0.decryptAes(cipherText1);
      assert.notEqual(msg.toString(), message.toString());
    }
  });

  it("Should correctly generate UTXO prefix viewing key", () => {
    const salt = "PREFIX_VIEWING_SALT";
    const expectedOutput: Uint8Array = new Uint8Array([
      234, 101, 252, 191, 221, 162, 81, 61, 96, 127, 241, 157, 190, 48, 250,
      147, 52, 212, 35, 226, 126, 246, 241, 98, 248, 163, 63, 9, 66, 56, 170,
      178,
    ]);
    const currentOutput = k0.getUtxoPrefixViewingKey(salt);
    return expect(currentOutput).to.eql(expectedOutput);
  });

  it("Should fail to generate UTXO prefix viewing key", () => {
    const salt = "PREFIX_VIEWING_SALT";
    // Made the expected output incorrect to make the test fail
    const expectedOutput: Uint8Array = new Uint8Array([
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0,
    ]);
    const currentOutput = k0.getUtxoPrefixViewingKey(salt);
    return expect(currentOutput).to.not.eql(expectedOutput);
  });

  it("Should correctly generate UTXO prefix hash", () => {
    const commitmentHash = new Uint8Array(32).fill(1);
    const prefixLength = 4;
    const expectedOutput: Uint8Array = new Uint8Array([55, 154, 4, 63]);
    const currentOutput = k0.generateUtxoPrefixHash(
      commitmentHash,
      prefixLength,
    );
    return expect(currentOutput).to.eql(expectedOutput);
  });

  it("Should fail UTXO prefix hash generation test for wrong expected output", () => {
    const commitmentHash = new Uint8Array(32).fill(1);
    const prefix_length = 4;
    const incorrectExpectedOutput: Uint8Array = new Uint8Array([1, 2, 3, 4]);
    const currentOutput = k0.generateUtxoPrefixHash(
      commitmentHash,
      prefix_length,
    );
    return expect(currentOutput).to.not.eql(incorrectExpectedOutput);
  });
});

describe("Test Account Errors", () => {
  let poseidon: Poseidon, k0: Account;
  before(async () => {
    poseidon = await Poseidon.getInstance();
    k0 = new Account({ poseidon, seed: seed32() });
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

  it("ENCRYPTION_PRIVATE_KEY_UNDEFINED", async () => {
    expect(() => {
      // @ts-ignore
      Account.fromPrivkey(
        poseidon,
        bs58.encode(k0.privkey.toArrayLike(Buffer, "be", 32)),
      );
    })
      .to.throw(AccountError)
      .includes({
        code: AccountErrorCode.ENCRYPTION_PRIVATE_KEY_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("AES_SECRET_UNDEFINED", () => {
    const { privateKey, encryptionPrivateKey } = k0.getPrivateKeys();

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
    const pubKey = k0.getPublicKey();

    const account = Account.fromPubkey(pubKey, poseidon);
    await chai.assert.isRejected(
      account.getEddsaPublicKey(),
      AccountErrorCode.POSEIDON_EDDSA_KEYPAIR_UNDEFINED,
    );
  });

  it("POSEIDON_EDDSA_KEYPAIR_UNDEFINED signEddsa", async () => {
    const pubKey = k0.getPublicKey();

    const account = Account.fromPubkey(pubKey, poseidon);
    await chai.assert.isRejected(
      account.signEddsa("123123"),
      AccountErrorCode.POSEIDON_EDDSA_KEYPAIR_UNDEFINED,
    );
  });
});
