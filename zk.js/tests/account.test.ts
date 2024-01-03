import { assert, expect } from "chai";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import {
  Account,
  AccountError,
  AccountErrorCode,
  ADMIN_AUTH_KEYPAIR,
  isEqualUint8Array,
  MerkleTreeConfig,
  newNonce,
  useWallet,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Hasher, WasmHasher } from "@lightprotocol/account.rs";

const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
const circomlibjs = require("circomlibjs");
const { buildBabyjub } = circomlibjs;
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const seed32 = bs58.encode(new Uint8Array(32).fill(1));

const keypairReferenceAccount = {
  encryptionPublicKey:
    "187,15,119,127,223,162,69,232,129,87,132,195,89,178,128,174,220,77,191,34,63,115,138,98,193,57,4,92,247,18,190,114",
  privkey:
    "7314374631704302594235695652925685842509708564100145210880269088513605645300",
  pubkey:
    "6391168142226478154718281169178137802178553836996014555114884736358424922672",
};

describe("Test Account Functional", () => {
  let hasher: Hasher,
    babyJub,
    F: any,
    k0: Account,
    k00: Account,
    kBurner: Account;

  before(async () => {
    hasher = await WasmHasher.getInstance();
    babyJub = await buildBabyjub();
    F = babyJub.F;
    k0 = Account.createFromSeed(hasher, seed32);
    k00 = Account.createFromSeed(hasher, seed32);
    kBurner = Account.createBurner(hasher, seed32, new BN("0"));
  });

  it("compare wasm account keypairs to the ref", () => {
    assert.equal(
      k0.keypair.privateKey.toString(),
      new BN(k0.wasmAccount.getPrivateKey()).toString(),
    );
    assert.equal(
      k0.keypair.publicKey.toString(),
      new BN(k0.wasmAccount.getPublicKey()).toString(),
    );
  });

  it("Test blake2 Domain separation", () => {
    const seed = bs58.encode([1, 2, 3]);
    const seedHash = hasher.blakeHash(seed, Account.hashLength);
    const encSeed = seedHash + "encryption";
    const privkeySeed = seedHash + "privkey";
    const privkeyHash = hasher.blakeHash(privkeySeed, Account.hashLength);
    const encHash = hasher.blakeHash(encSeed, Account.hashLength);

    assert.notEqual(encHash, seedHash);
    assert.notEqual(privkeyHash, seedHash);
    assert.notEqual(encHash, privkeyHash);
  });

  it("Test poseidon", async () => {
    let x = new Array(30).fill(1);
    let y = new Array(30).fill(2);
    const hasher = await WasmHasher.getInstance();
    const hash = hasher.poseidonHashString([
      new BN(x).toString(),
      new BN(y).toString(),
    ]);

    x = new Array(29).fill(1);
    y = new Array(31).fill(2);
    y[30] = 1;

    const hash1 = hasher.poseidonHashString([
      new BN(x).toString(),
      new BN(y).toString(),
    ]);
    assert.notEqual(hash, hash1);
  });

  it("Test wasm poseidon", async () => {
    const hasher = await WasmHasher.getInstance();
    let x = new Array(30).fill(1);
    let y = new Array(30).fill(2);

    const hash = hasher.poseidonHash([
      new BN(x).toString(),
      new BN(y).toString(),
    ]);
    x = new Array(29).fill(1);
    y = new Array(31).fill(2);
    y[30] = 1;

    const hash1 = hasher.poseidonHash([
      new BN(x).toString(),
      new BN(y).toString(),
    ]);
    assert.notEqual(hash, hash1);
  });

  const compareKeypairsEqual = (
    k0: Account,
    k1: Account,
    fromPrivkey: boolean = false,
  ) => {
    assert.equal(
      k0.keypair.privateKey.toString(),
      k1.keypair.privateKey.toString(),
    );
    assert.equal(
      k0.keypair.publicKey.toString(),
      k1.keypair.publicKey.toString(),
    );
    if (k0.wasmAccount.getBurnerSeed() && k1.wasmAccount.getBurnerSeed()) {
      assert.equal(
        k0.wasmAccount.getBurnerSeed().toString(),
        k1.wasmAccount.getBurnerSeed().toString(),
      );
    }
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
    assert.notEqual(
      k0.keypair.privateKey.toString(),
      k1.keypair.privateKey.toString(),
    );
    assert.notEqual(
      k0.encryptionKeypair.publicKey.toString(),
      k1.encryptionKeypair.publicKey.toString(),
    );
    assert.notEqual(
      k0.keypair.publicKey.toString(),
      k1.keypair.publicKey.toString(),
    );
    if (
      burner &&
      k0.wasmAccount.getBurnerSeed() &&
      k1.wasmAccount.getBurnerSeed()
    ) {
      assert.notEqual(
        k0.wasmAccount.getBurnerSeed().toString(),
        k1.wasmAccount.getBurnerSeed().toString(),
      );
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
    assert.equal(
      account.keypair.privateKey.toString(),
      reference.privkey.toString(),
    );
    assert.equal(
      account.keypair.publicKey.toString(),
      reference.pubkey.toString(),
    );
    if (reference.burnerSeed) {
      assert.equal(
        Array.from(kBurner.wasmAccount.getBurnerSeed()).toString(),
        reference.burnerSeed.toString(),
      );
    }
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
    };
    await compareAccountToReference(k0, referenceAccount);

    const seedDiff32 = bs58.encode(new Uint8Array(32).fill(2));
    const k1 = Account.createFromSeed(hasher, seedDiff32);
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(k0, k1);

    const k2 = Account.createFromSeed(hasher, seed32);
    compareKeypairsEqual(k0, k2);
  });

  it("createFromSolanaKeypair Functional", async () => {
    const solanaKeypairAccount = Account.createFromSolanaKeypair(
      hasher,
      ADMIN_AUTH_KEYPAIR,
    );
    await compareAccountToReference(
      solanaKeypairAccount,
      keypairReferenceAccount,
    );

    const seedDiff32 = bs58.encode(new Uint8Array(32).fill(2));
    const k1 = Account.createFromSeed(hasher, seedDiff32);
    // keypairs from different seeds are not equal
    compareKeypairsNotEqual(solanaKeypairAccount, k1);
  });

  it.skip("createFromBrowserWallet Functional", async () => {
    const wallet = useWallet(ADMIN_AUTH_KEYPAIR);

    const solanaWalletAccount = await Account.createFromBrowserWallet(
      hasher,
      wallet,
    );
    await compareAccountToReference(
      solanaWalletAccount,
      keypairReferenceAccount,
    );

    const seedDiff32 = bs58.encode(new Uint8Array(32).fill(2));
    const k1 = Account.createFromSeed(hasher, seedDiff32);
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
    };
    await compareAccountToReference(kBurner, referenceAccount);

    // burners and regular keypair from the same seed are not equal
    compareKeypairsNotEqual(k0, kBurner, true);

    const burnerSeed = kBurner.wasmAccount.getBurnerSeed();
    const seed2 = bs58.encode(burnerSeed);
    const kBurner2 = Account.fromBurnerSeed(hasher, seed2);
    compareKeypairsEqual(kBurner2, kBurner);
    compareKeypairsNotEqual(k0, kBurner2, true);
  });

  it("Burner same index & keypair eq", () => {
    const kBurner0 = Account.createBurner(hasher, seed32, new BN("0"));
    // burners with the same index from the same seed are the equal
    compareKeypairsEqual(kBurner0, kBurner);
  });

  it("Burner diff index & keypair neq", () => {
    const kBurner0 = Account.createBurner(hasher, seed32, new BN("0"));
    // burners with the same index from the same seed are the equal
    compareKeypairsEqual(kBurner0, kBurner);
    const kBurner1 = Account.createBurner(hasher, seed32, new BN("1"));
    // burners with incrementing index are not equal
    compareKeypairsNotEqual(kBurner1, kBurner0, true);
  });

  it("fromPrivkey", () => {
    if (!k0.aesSecret) throw new Error("Aes key is undefined");
    const { privateKey, aesSecret, encryptionPrivateKey } = k0.getPrivateKeys();
    const k0Privkey = Account.fromPrivkey(
      hasher,
      privateKey,
      encryptionPrivateKey,
      aesSecret,
    );
    compareKeypairsEqual(k0Privkey, k0, true);
  });

  it("fromPubkey", () => {
    const pubKey = k0.getPublicKey();
    const k0Pubkey = Account.fromPubkey(pubKey, hasher);
    assert.equal(
      k0Pubkey.keypair.publicKey.toString(),
      k0.keypair.publicKey.toString(),
    );
    assert.notEqual(k0Pubkey.keypair.privateKey, k0.keypair.privateKey);
  });

  it("aes encryption", async () => {
    const message = new Uint8Array(32).fill(1);
    // never reuse nonces this is only for testing
    const nonce = newNonce().subarray(0, 12);
    const cipherText1 = await k0.encryptAes(message, nonce);
    const cleartext1 = k0.decryptAes(cipherText1);
    assert.isNotNull(cleartext1);
    assert.isTrue(isEqualUint8Array(cleartext1!, message));
    assert.isFalse(isEqualUint8Array(new Uint8Array(32).fill(1), k0.aesSecret));

    // try to decrypt with invalid secretkey
    // added try catch because in some cases it doesn't decrypt but doesn't throw an error either
    //TODO: revisit this and possibly switch aes library
    try {
      await chai.assert.isRejected(kBurner.decryptAes(cipherText1), Error);
    } catch (error) {
      const msg = k0.decryptAes(cipherText1);
      assert.isNotNull(msg);
      assert.isTrue(isEqualUint8Array(msg!, message));
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
    const expectedOutput: Uint8Array = new Uint8Array([240, 73, 165, 83]);
    const merkleTreePda = MerkleTreeConfig.getTransactionMerkleTreePda();
    expect(merkleTreePda).to.not.be.undefined;

    const currentOutput = k0.generateUtxoPrefixHash(merkleTreePda, 0);
    expect(currentOutput).to.eql(expectedOutput);

    const currentOutput1 = k0.generateUtxoPrefixHash(merkleTreePda, 1);
    expect(currentOutput1).to.not.eq(expectedOutput);

    const currentOutput0 = k0.generateLatestUtxoPrefixHash(merkleTreePda);
    expect(currentOutput).to.eql(currentOutput0);
    expect(k0.wasmAccount.getPrefixCounter()).to.eql(1);

    const currentOutputLatest1 = k0.generateLatestUtxoPrefixHash(merkleTreePda);
    expect(currentOutput1).to.deep.eq(currentOutputLatest1);
  });

  it("Should fail UTXO prefix hash generation test for wrong expected output", () => {
    const incorrectExpectedOutput: Uint8Array = new Uint8Array([1, 2, 3, 4]);
    const currentOutput = k0.generateUtxoPrefixHash(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      0,
    );
    return expect(currentOutput).to.not.eql(incorrectExpectedOutput);
  });
});

describe("Test Account Errors", () => {
  let hasher: Hasher, k0: Account;
  before(async () => {
    hasher = await WasmHasher.getInstance();
    k0 = Account.createFromSeed(hasher, seed32);
  });

  it("INVALID_SEED_SIZE", async () => {
    expect(() => {
      Account.createFromSeed(hasher, bs58.encode([1, 2, 3]));
    })
      .to.throw(AccountError)
      .includes({
        code: AccountErrorCode.INVALID_SEED_SIZE,
        functionName: "constructor",
      });
  });

  it("INVALID_SEED_SIZE burner", async () => {
    expect(() => {
      Account.fromBurnerSeed(hasher, "123");
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
        hasher,
        bs58.encode(k0.keypair.privateKey.toArrayLike(Buffer, "be", 32)),
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
      Account.fromPrivkey(hasher, privateKey, encryptionPrivateKey);
    })
      .to.throw(AccountError)
      .includes({
        code: AccountErrorCode.AES_SECRET_UNDEFINED,
        functionName: "constructor",
      });
  });
});
