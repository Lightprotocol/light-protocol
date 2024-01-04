import { sign } from "tweetnacl";
import { BN, Idl, utils } from "@coral-xyz/anchor";
import {
  AccountError,
  AccountErrorCode,
  BN_0,
  SIGN_MESSAGE,
  STANDARD_SHIELDED_PRIVATE_KEY,
  STANDARD_SHIELDED_PUBLIC_KEY,
  TransactionErrorCode,
  Utxo,
  UtxoErrorCode,
  Wallet,
} from "./index";
import { Hasher, WasmAccount } from "@lightprotocol/account.rs";
import { Keypair, PublicKey } from "@solana/web3.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Result } from "./types";
import { Prover } from "@lightprotocol/prover.js";

const nacl = require("tweetnacl");

// TODO: add fromPubkeyString()
export class Account {
  wasmAccount: WasmAccount;

  public get keypair(): { privateKey: BN; publicKey: BN } {
    return {
      privateKey: new BN(this.wasmAccount.getPrivateKey()),
      publicKey: new BN(this.wasmAccount.getPublicKey()),
    };
  }

  get encryptionKeypair(): { privateKey: Uint8Array; publicKey: Uint8Array } {
    return {
      privateKey: this.wasmAccount.getEncryptionPrivateKey(),
      publicKey: this.wasmAccount.getEncryptionPublicKey(),
    };
  }

  get solanaPublicKey(): PublicKey {
    return this.wasmAccount.getSolanaPublicKey();
  }

  get aesSecret(): Uint8Array {
    return this.wasmAccount.getAesSecret();
  }

  getPrivateKeys(): {
    privateKey: string;
    encryptionPrivateKey: string;
    aesSecret: string;
  } {
    if (!this.aesSecret) {
      throw new AccountError(
        AccountErrorCode.AES_SECRET_UNDEFINED,
        "getPrivateKeys",
      );
    }
    return {
      privateKey: bs58.encode(this.keypair.privateKey.toArray("be", 32)),
      encryptionPrivateKey: bs58.encode(this.encryptionKeypair.privateKey),
      aesSecret: bs58.encode(this.aesSecret),
    };
  }

  static readonly hashLength = 32;

  private constructor({
    hasher,
    seed = bs58.encode(nacl.randomBytes(32)),
    burner = false,
    burnerIndex = "",
    burnerSeed = false,
    privateKey,
    publicKey,
    encryptionPublicKey,
    encryptionPrivateKey,
    aesSecret,
    solanaPublicKey,
    prefixCounter,
  }: {
    hasher: Hasher;
    seed?: string;
    burner?: boolean;
    burnerIndex?: string;
    burnerSeed?: boolean;
    privateKey?: BN;
    publicKey?: BN;
    encryptionPublicKey?: Uint8Array;
    encryptionPrivateKey?: Uint8Array;
    aesSecret?: Uint8Array;
    solanaPublicKey?: PublicKey;
    prefixCounter?: BN;
  }) {
    if (aesSecret && !privateKey) {
      this.wasmAccount = hasher.aesAccount(aesSecret);
    }
    // creates a burner utxo by using the index for domain separation
    else if (burner) {
      if (!seed) {
        throw new AccountError(
          AccountErrorCode.SEED_UNDEFINED,
          "constructor",
          "seed is required to create a burner account",
        );
      }
      if (bs58.decode(seed).length !== 32) {
        throw new AccountError(
          AccountErrorCode.INVALID_SEED_SIZE,
          "constructor",
          "seed too short length less than 32",
        );
      }
      if (burnerSeed) {
        this.wasmAccount = hasher.burnerSeedAccount(seed);
      } else {
        this.wasmAccount = hasher.burnerAccount(seed, burnerIndex);
      }
    } else if (privateKey && encryptionPrivateKey && aesSecret) {
      this.wasmAccount = hasher.privateKeyAccount(
        Uint8Array.from([...privateKey.toArray("be", 32)]),
        encryptionPrivateKey,
        aesSecret,
      );
    } else if (publicKey) {
      this.wasmAccount = hasher.publicKeyAccount(
        Uint8Array.from([...publicKey.toArray("be", 32)]),
        encryptionPublicKey,
      );
    } else {
      if (!seed) {
        throw new AccountError(
          AccountErrorCode.SEED_UNDEFINED,
          "constructor",
          "seed is required to create an account",
        );
      }
      if (seed.length < 32) {
        throw new AccountError(
          AccountErrorCode.INVALID_SEED_SIZE,
          "constructor",
          "Seed length is less than 32",
        );
      }
      this.wasmAccount = hasher.seedAccount(seed);
    }

    this.wasmAccount.setSolanaPublicKey(solanaPublicKey);
    this.wasmAccount.setPrefixCounter(prefixCounter ?? BN_0);
  }

  // constructors

  static random(hasher: Hasher): Account {
    return new Account({ hasher });
  }
  static createFromSeed(hasher: Hasher, seed: string): Account {
    return new Account({ hasher, seed });
  }

  static createFromSolanaKeypair(hasher: Hasher, keypair: Keypair): Account {
    const encodedMessage = utils.bytes.utf8.encode(SIGN_MESSAGE);
    const signature: Uint8Array = sign.detached(
      encodedMessage,
      keypair.secretKey,
    );
    return new Account({
      hasher,
      seed: bs58.encode(signature),
      solanaPublicKey: keypair.publicKey,
    });
  }

  static async createFromBrowserWallet(
    hasher: Hasher,
    wallet: Wallet,
  ): Promise<Account> {
    const encodedMessage = utils.bytes.utf8.encode(SIGN_MESSAGE);
    const signature: Uint8Array = await wallet.signMessage(encodedMessage);
    return new Account({
      hasher,
      seed: bs58.encode(signature),
      solanaPublicKey: wallet.publicKey,
    });
  }

  static createBurner(hasher: Hasher, seed: string, burnerIndex: BN): Account {
    return new Account({
      hasher,
      seed,
      burner: true,
      burnerIndex: burnerIndex.toString(),
    });
  }

  static fromBurnerSeed(hasher: Hasher, seed: string): Account {
    return new Account({ hasher, seed, burnerSeed: true, burner: true });
  }

  static fromPrivkey(
    hasher: Hasher,
    privateKey: string,
    encryptionPrivateKey: string,
    aesSecret: string,
  ): Account {
    if (!privateKey) {
      throw new AccountError(
        AccountErrorCode.PRIVATE_KEY_UNDEFINED,
        "constructor",
      );
    }
    if (!encryptionPrivateKey) {
      throw new AccountError(
        AccountErrorCode.ENCRYPTION_PRIVATE_KEY_UNDEFINED,
        "constructor",
      );
    }

    if (!aesSecret) {
      throw new AccountError(
        AccountErrorCode.AES_SECRET_UNDEFINED,
        "constructor",
      );
    }

    const privkey = new BN(bs58.decode(privateKey));

    return new Account({
      hasher,
      privateKey: privkey,
      encryptionPrivateKey: bs58.decode(encryptionPrivateKey),
      aesSecret: bs58.decode(aesSecret),
    });
  }

  static fromPubkey(publicKey: string, hasher: Hasher): Account {
    const decoded = bs58.decode(publicKey);
    if (decoded.length != 64)
      throw new AccountError(
        AccountErrorCode.INVALID_PUBLIC_KEY_SIZE,
        "fromPubkey",
        `Invalid length: ${decoded.length} bytes. Expected 64 bytes for publicKey, 
          where the first 32 are the shielded key and the second 32 are the encryption key.`,
      );

    const pubKey = new BN(decoded.subarray(0, 32), undefined, "be");
    return new Account({
      publicKey: pubKey,
      encryptionPublicKey: decoded.subarray(32, 64),
      hasher,
    });
  }

  // instance methods
  sign(commitment: string, merklePath: number): BN {
    return new BN(this.wasmAccount.sign(commitment, merklePath));
  }

  getAesUtxoViewingKey(
    merkleTreePdaPublicKey: PublicKey,
    salt: string,
  ): Uint8Array {
    return this.wasmAccount.getAesUtxoViewingKey(
      merkleTreePdaPublicKey.toBytes(),
      salt,
    );
  }

  getUtxoPrefixViewingKey(salt: string): Uint8Array {
    return this.wasmAccount.getUtxoPrefixViewingKey(salt);
  }

  generateLatestUtxoPrefixHash(merkleTreePublicKey: PublicKey): Uint8Array {
    return this.wasmAccount.generateLatestUtxoPrefixHash(
      merkleTreePublicKey.toBytes(),
    );
  }

  generateUtxoPrefixHash(
    merkleTreePublicKey: PublicKey,
    prefixCounter: number,
  ) {
    return this.wasmAccount.generateUtxoPrefixHash(
      merkleTreePublicKey.toBytes(),
      prefixCounter,
    );
  }

  getPublicKey(): string {
    return this.wasmAccount.getCombinedPublicKey();
  }

  /**
   * Encrypts UTXO bytes with UTXO viewing key and iv from commitment.
   * @param messageBytes - The bytes message to be encrypted.
   * @param merkleTreePdaPublicKey - The public key used in encryption.
   * @param commitment - The commitment used as the Initialization Vector (iv).
   * @returns A promise that resolves to the encrypted Uint8Array.
   */
  encryptAesUtxo(
    messageBytes: Uint8Array,
    merkleTreePdaPublicKey: PublicKey,
    commitment: Uint8Array,
  ): Uint8Array {
    return this.wasmAccount.encryptAesUtxo(
      messageBytes,
      merkleTreePdaPublicKey.toBytes(),
      commitment,
    );
  }

  /**
   * Encrypts bytes with aes secret key.
   * @param messageBytes - The bytes to be encrypted.
   * @param iv12 - Optional Initialization Vector (iv), 12 random bytes by default.
   * @returns A Uint8Array of encrypted bytes with the iv as the first 16 bytes of the cipher text.
   */
  async encryptAes(
    messageBytes: Uint8Array,
    iv12: Uint8Array = nacl.randomBytes(12),
  ) {
    if (!this.aesSecret) {
      throw new AccountError(UtxoErrorCode.AES_SECRET_UNDEFINED, "encryptAes");
    }

    if (iv12.length != 12) {
      throw new AccountError(
        UtxoErrorCode.INVALID_NONCE_LENGTH,
        "encryptAes",
        `Required iv length 12, provided ${iv12.length}`,
      );
    }

    return this.wasmAccount.encryptAes(messageBytes, iv12);
  }

  /**
   * Decrypts encrypted UTXO bytes with UTXO viewing key and iv from commitment.
   * @param encryptedBytes - The encrypted bytes to be decrypted.
   * @param merkleTreePdaPublicKey - The public key used in decryption.
   * @param commitment - The commitment used as the Initialization Vector (iv).
   * @returns A promise that resolves to a Result object containing the decrypted Uint8Array or an error if the decryption fails.
   */
  decryptAesUtxo(
    encryptedBytes: Uint8Array,
    merkleTreePdaPublicKey: PublicKey,
    commitment: Uint8Array,
  ): Result<Uint8Array | null, Error> {
    // Check if account secret key is available for decrypting using AES
    if (!this.aesSecret) {
      throw new AccountError(
        UtxoErrorCode.AES_SECRET_UNDEFINED,
        "decryptAesUtxo",
      );
    }

    try {
      const decryptedAesUtxo = this.wasmAccount.decryptAesUtxo(
        encryptedBytes,
        merkleTreePdaPublicKey.toBytes(),
        commitment,
      );
      return Result.Ok(decryptedAesUtxo);
    } catch (e: any) {
      return Result.Err(Error(e.toString()));
    }
  }

  /**
   * Decrypts AES encrypted bytes, the iv is expected to be the first 16 bytes.
   * @param encryptedBytes - The AES encrypted bytes to be decrypted.
   * @returns A promise that resolves to a Result containing the decrypted Uint8Array or null in case of an error.
   * @throws Will throw an error if the aesSecret is undefined.
   */
  decryptAes(encryptedBytes: Uint8Array): Uint8Array | null {
    if (!this.aesSecret) {
      throw new AccountError(UtxoErrorCode.AES_SECRET_UNDEFINED, "decryptAes");
    }
    return this.wasmAccount.decryptAes(encryptedBytes);
  }

  /**
   * Decrypts encrypted UTXO bytes.
   * @param ciphertext - The encrypted bytes to be decrypted.
   * @param commitment - The commitment used to generate the nonce.
   * @returns A promise that resolves to a Result containing the decrypted Uint8Array or null in case of an error.
   */
  decryptNaclUtxo(
    ciphertext: Uint8Array,
    commitment: Uint8Array,
  ): Result<Uint8Array | null, Error> {
    try {
      const decryptedUtxo = this.wasmAccount.decryptNaclUtxo(
        ciphertext,
        commitment,
      );
      return Result.Ok(decryptedUtxo);
    } catch (e: any) {
      return Result.Err(Error(e.toString()));
    }
  }

  private addPrivateKey(proofInput: any, inputUtxos: Utxo[]) {
    proofInput["inPrivateKey"] = inputUtxos.map((utxo: Utxo) => {
      if (utxo.publicKey.eq(this.keypair.publicKey)) {
        return this.keypair.privateKey;
      }
      if (STANDARD_SHIELDED_PUBLIC_KEY.eq(utxo.publicKey)) {
        return STANDARD_SHIELDED_PRIVATE_KEY;
      }
    });
  }

  async getProofInternal({
    firstPath,
    verifierIdl,
    circuitName,
    proofInput,
    addPrivateKey,
    enableLogging,
    inputUtxos,
  }: {
    firstPath: string;
    verifierIdl: Idl;
    circuitName?: string;
    proofInput: any;
    addPrivateKey?: boolean;
    enableLogging?: boolean;
    inputUtxos?: Utxo[];
  }) {
    if (!proofInput)
      throw new AccountError(
        TransactionErrorCode.PROOF_INPUT_UNDEFINED,
        "getProofInternal",
      );
    if (!verifierIdl)
      throw new AccountError(
        TransactionErrorCode.NO_PARAMETERS_PROVIDED,
        "getProofInternal",
        "verifierIdl is missing in TransactionParameters",
      );

    if (addPrivateKey && !inputUtxos) {
      throw new AccountError(
        TransactionErrorCode.NO_VERIFIER_IDL_PROVIDED,
        "getProofInternal",
        "verifierIdl is missing in TransactionParameters",
      );
    }
    if (addPrivateKey && inputUtxos) {
      this.addPrivateKey(proofInput, inputUtxos);
    }
    const prover = new Prover(verifierIdl, firstPath, circuitName);
    await prover.addProofInputs(proofInput);
    const prefix = `\x1b[37m[${new Date(Date.now()).toISOString()}]\x1b[0m`;
    const logMsg = `${prefix} Proving ${verifierIdl.name} circuit`;
    if (enableLogging) {
      console.time(logMsg);
    }

    let parsedProof, parsedPublicInputs;
    try {
      const result = await prover.fullProveAndParse();
      parsedProof = result.parsedProof;
      parsedPublicInputs = result.parsedPublicInputs;
    } catch (error: any) {
      throw new AccountError(
        TransactionErrorCode.PROOF_GENERATION_FAILED,
        "getProofInternal",
        error,
      );
    }
    if (enableLogging) {
      console.timeEnd(logMsg);
    }

    const res = await prover.verify();
    if (!res) {
      throw new AccountError(
        TransactionErrorCode.INVALID_PROOF,
        "getProofInternal",
      );
    }
    const parsedPublicInputsObject =
      prover.parsePublicInputsFromArray(parsedPublicInputs);
    return { parsedProof, parsedPublicInputsObject };
  }
}
