import { box, sign } from "tweetnacl";
import { BN, utils } from "@coral-xyz/anchor";
import {
  AccountError,
  AccountErrorCode,
  BN_0,
  CONSTANT_SECRET_AUTHKEY,
  setEnvironment,
  SIGN_MESSAGE,
  STANDARD_SHIELDED_PRIVATE_KEY,
  STANDARD_SHIELDED_PUBLIC_KEY,
  TransactionErrorCode,
  TransactionParameters,
  truncateToCircuit,
  Utxo,
  UtxoErrorCode,
  Wallet,
} from "./index";
import { Poseidon, blake } from "@lightprotocol/account.rs";
import { Keypair, PublicKey } from "@solana/web3.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Result } from "./types";
import { Prover } from "@lightprotocol/prover.js";

const nacl = require("tweetnacl");

const { encrypt, decrypt } = require("ethereum-cryptography/aes");
const ffjavascript = require("ffjavascript");
const buildEddsa = require("circomlibjs").buildEddsa;

// TODO: add fromPubkeyString()
export class Account {
  /**
   * Initialize a new shielded account. Generates a random private key if not defined
   *
   * @param {BN} privkey
   * @param {BN} pubkey
   */
  privkey: BN;
  pubkey: BN;
  encryptionKeypair: nacl.BoxKeyPair;
  burnerSeed: Uint8Array;
  // keypair for eddsa poseidon signatures
  poseidonEddsaKeypair?: {
    publicKey?: [Uint8Array, Uint8Array];
    privateKey: Uint8Array;
  };
  eddsa: any;
  aesSecret?: Uint8Array;
  hashingSecret?: Uint8Array;
  solanaPublicKey?: PublicKey;

  static readonly hashLength = 32;

  constructor({
    poseidon,
    seed = bs58.encode(nacl.randomBytes(32)),
    burner = false,
    privateKey,
    publicKey,
    poseidonEddsaPrivateKey,
    eddsa,
    encryptionPublicKey,
    encryptionPrivateKey,
    aesSecret,
    solanaPublicKey,
  }: {
    poseidon: Poseidon;
    seed?: string;
    burner?: boolean;
    privateKey?: BN;
    publicKey?: BN;
    poseidonEddsaPrivateKey?: Uint8Array;
    eddsa?: any;
    encryptionPublicKey?: Uint8Array;
    encryptionPrivateKey?: Uint8Array;
    aesSecret?: Uint8Array;
    solanaPublicKey?: PublicKey;
  }) {
    this.solanaPublicKey = solanaPublicKey;
    this.burnerSeed = new Uint8Array();
    if (aesSecret && !privateKey) {
      this.aesSecret = aesSecret;
      this.privkey = BN_0;
      this.pubkey = BN_0;
      this.encryptionKeypair = {
        publicKey: new Uint8Array(32),
        secretKey: new Uint8Array(32),
      };
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
      const decoded_seed = bs58.decode(seed);
      if (decoded_seed.length !== 32) {
        throw new AccountError(
          AccountErrorCode.INVALID_SEED_SIZE,
          "constructor",
          "Seed length assertion failed. Expected 32, got ${decoded_seed.length}.",
        );
      }
      // burnerSeed can be shared since hash cannot be inverted - only share this for app utxos
      // sharing the burnerSeed saves 32 bytes in on-chain data if it is required to share both
      // the encryption and private key of an utxo
      this.burnerSeed = new BN(bs58.decode(seed)).toArrayLike(Buffer, "be", 32);
      this.privkey = Account.generateShieldedPrivateKey(seed, poseidon);
      this.encryptionKeypair = Account.getEncryptionKeyPair(seed);
      this.pubkey = Account.generateShieldedPublicKey(this.privkey, poseidon);
      this.poseidonEddsaKeypair = Account.getEddsaPrivateKey(
        this.burnerSeed.toString(),
      );
      this.aesSecret = Account.generateSecret(
        this.burnerSeed.toString(),
        "aes",
      );
      this.hashingSecret = Account.generateSecret(
        this.burnerSeed.toString(),
        "hashing",
      );
    } else if (privateKey) {
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

      this.aesSecret = aesSecret;

      this.privkey = privateKey;
      this.pubkey = Account.generateShieldedPublicKey(this.privkey, poseidon);
      if (poseidonEddsaPrivateKey) {
        this.poseidonEddsaKeypair = { privateKey: poseidonEddsaPrivateKey };
      }
      this.encryptionKeypair =
        nacl.box.keyPair.fromSecretKey(encryptionPrivateKey);
    } else if (publicKey) {
      this.pubkey = publicKey;
      this.privkey = BN_0;
      this.encryptionKeypair = {
        publicKey: encryptionPublicKey ? encryptionPublicKey : new Uint8Array(),
        secretKey: new Uint8Array(),
      };
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
      this.encryptionKeypair = Account.getEncryptionKeyPair(seed);
      this.privkey = Account.generateShieldedPrivateKey(seed, poseidon);
      this.pubkey = Account.generateShieldedPublicKey(this.privkey, poseidon);
      this.poseidonEddsaKeypair = Account.getEddsaPrivateKey(seed);
      this.aesSecret = Account.generateSecret(seed, "aes");
      this.hashingSecret = Account.generateSecret(seed, "hashing");
    }
    this.eddsa = eddsa;
  }

  async getEddsaPublicKey(eddsa?: any): Promise<[Uint8Array, Uint8Array]> {
    if (!this.poseidonEddsaKeypair) {
      throw new AccountError(
        AccountErrorCode.POSEIDON_EDDSA_KEYPAIR_UNDEFINED,
        "getEddsaPublicKey",
        "poseidonEddsaKeypair.privateKey undefined",
      );
    }

    if (!this.eddsa) {
      if (eddsa) {
        this.eddsa = eddsa;
      } else {
        this.eddsa = await buildEddsa();
      }
    }

    this.poseidonEddsaKeypair.publicKey = this.eddsa.prv2pub(
      this.poseidonEddsaKeypair?.privateKey,
    );
    if (!this.poseidonEddsaKeypair.publicKey) {
      throw new AccountError(
        AccountErrorCode.POSEIDON_EDDSA_GET_PUBKEY_FAILED,
        "signEddsa",
        "poseidonEddsaKeypair.privateKey undefined",
      );
    }
    return this.poseidonEddsaKeypair.publicKey;
  }

  static getEddsaPrivateKey(seed: string) {
    const privkeySeed = seed + "poseidonEddsaKeypair";
    return {
      publicKey: undefined,
      privateKey: blake(privkeySeed, Account.hashLength),
    };
  }

  encryptionPublicKeyToBytes() {
    return new BN(this.encryptionKeypair.publicKey).toArrayLike(
      Buffer,
      "be",
      32,
    );
  }

  // TODO: Add check for uint8array to be well formed
  async signEddsa(msg: string | Uint8Array, eddsa?: any): Promise<Uint8Array> {
    if (!this.poseidonEddsaKeypair) {
      throw new AccountError(
        AccountErrorCode.POSEIDON_EDDSA_KEYPAIR_UNDEFINED,
        "signEddsa",
        "poseidonEddsaKeypair.privateKey undefined",
      );
    }

    if (!this.eddsa) {
      if (eddsa) {
        this.eddsa = eddsa;
      } else {
        this.eddsa = await buildEddsa();
      }
    }

    if (typeof msg == "string") {
      return this.eddsa.packSignature(
        this.eddsa.signPoseidon(
          this.poseidonEddsaKeypair.privateKey,
          this.eddsa.F.e(ffjavascript.Scalar.e(msg)),
        ),
      );
    } else {
      return this.eddsa.packSignature(
        this.eddsa.signPoseidon(this.poseidonEddsaKeypair.privateKey, msg),
      );
    }
  }

  sign(poseidon: Poseidon, commitment: string, merklePath: number): BN {
    return new BN(
      poseidon.hash([
        this.privkey.toString(),
        commitment.toString(),
        merklePath.toString(),
      ]),
    );
  }

  getAesUtxoViewingKey(
    merkleTreePdaPublicKey: PublicKey,
    salt: string,
  ): Uint8Array {
    return Account.generateSecret(
      this.aesSecret?.toString(),
      merkleTreePdaPublicKey.toBase58() + salt,
    );
  }

  getUtxoPrefixViewingKey(salt: string): Uint8Array {
    return Account.generateSecret(this.hashingSecret?.toString(), salt);
  }

  generateUtxoPrefixHash(commitmentHash: Uint8Array, prefixLength: number) {
    const input = Uint8Array.from([
      ...this.getUtxoPrefixViewingKey("hashing"),
      ...commitmentHash,
    ]);

    return blake(input, prefixLength);
  }

  static createBurner(poseidon: Poseidon, seed: string, index: BN): Account {
    if (bs58.decode(seed).length !== 32) {
      throw new AccountError(
        AccountErrorCode.INVALID_SEED_SIZE,
        "constructor",
        "seed too short length less than 32",
      );
    }
    const input = seed + "burnerSeed" + index.toString();
    const burnerSeed = blake(input, Account.hashLength);
    const burnerSeedString = bs58.encode(burnerSeed);
    return new Account({ poseidon, seed: burnerSeedString, burner: true });
  }

  static fromBurnerSeed(poseidon: Poseidon, burnerSeed: string): Account {
    return new Account({ poseidon, seed: burnerSeed, burner: true });
  }

  static fromPrivkey(
    poseidon: Poseidon,
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
      poseidon,
      privateKey: privkey,
      encryptionPrivateKey: bs58.decode(encryptionPrivateKey),
      aesSecret: bs58.decode(aesSecret),
    });
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
      privateKey: bs58.encode(this.privkey.toArray("be", 32)),
      encryptionPrivateKey: bs58.encode(this.encryptionKeypair.secretKey),
      aesSecret: bs58.encode(this.aesSecret),
    };
  }

  static fromPubkey(publicKey: string, poseidon: Poseidon): Account {
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
      poseidon,
    });
  }

  getPublicKey(): string {
    const concatPublicKey = new Uint8Array([
      ...this.pubkey.toArray("be", 32),
      ...this.encryptionKeypair.publicKey,
    ]);
    return bs58.encode(concatPublicKey);
  }

  static getEncryptionKeyPair(seed: string): nacl.BoxKeyPair {
    const encSeed = seed + "encryption";
    const encryptionPrivateKey = blake(encSeed, Account.hashLength);
    return nacl.box.keyPair.fromSecretKey(encryptionPrivateKey);
  }

  static generateShieldedPrivateKey(seed: string, poseidon: Poseidon): BN {
    const privateKeySeed = seed + "shielded";
    const blakeHash: Uint8Array = blake(privateKeySeed, Account.hashLength);

    // TODO(sergey): unify next 2 lines by setting blake hash length to 31 bytes
    // example: const blakeHash: Uint8Array = blake(privateKeySeed,31);
    const blakeHash31 = truncateToCircuit(blakeHash);
    const blakeHashBN = new BN(blakeHash31);
    return new BN(poseidon.hash([blakeHashBN.toString()]));
  }

  static generateSecret(seed?: string, domain?: string): Uint8Array {
    const input: string = `${seed}${domain}`;
    return blake(input, Account.hashLength);
  }

  static generateShieldedPublicKey(privateKey: BN, poseidon: Poseidon): BN {
    return new BN(poseidon.hash([privateKey.toString()]));
  }

  /**
   * Encrypts UTXO bytes with UTXO viewing key and iv from commitment.
   * @param messageBytes - The bytes message to be encrypted.
   * @param merkleTreePdaPublicKey - The public key used in encryption.
   * @param commitment - The commitment used as the Initialization Vector (iv).
   * @returns A promise that resolves to the encrypted Uint8Array.
   */
  async encryptAesUtxo(
    messageBytes: Uint8Array,
    merkleTreePdaPublicKey: PublicKey,
    commitment: Uint8Array,
  ): Promise<Uint8Array> {
    setEnvironment();
    const iv16 = commitment.subarray(0, 16);
    return this._encryptAes(
      messageBytes,
      this.getAesUtxoViewingKey(
        merkleTreePdaPublicKey,
        bs58.encode(commitment),
      ),
      iv16,
    );
  }



  /**
   * Encrypts bytes with aes secret key.
   * @param messageBytes - The bytes to be encrypted.
   * @param iv16 - Optional Initialization Vector (iv), 16 random bytes by default.
   * @returns A Uint8Array of encrypted bytes with the iv as the first 16 bytes of the cipher text.
   */
  async encryptAes(
    messageBytes: Uint8Array,
    iv16: Uint8Array = nacl.randomBytes(16),
  ) {
    const ciphertext = await encrypt(
      messageBytes,
      this.aesSecret,
      iv16,
      "aes-256-cbc",
      true,
    );
    return new Uint8Array([...iv16, ...ciphertext]);
  }



  /**
   * Private aes encryption method.
   * @private
   * @param messageBytes - The messageBytes to be encrypted.
   * @param secretKey - The secret key to be used for encryption.
   * @param iv16 - The Initialization Vector (iv) to be used for encryption.
   * @returns A promise that resolves to the encrypted Uint8Array.
   */
  private async _encryptAes(
    messageBytes: Uint8Array,
    secretKey: Uint8Array,
    iv16: Uint8Array,
  ) {
    if (iv16.length != 16)
      throw new AccountError(
        UtxoErrorCode.INVALID_NONCE_LENGTH,
        "encryptAes",
        `Required iv length 16, provided ${iv16.length}`,
      );

    return await encrypt(messageBytes, secretKey, iv16, "aes-256-cbc", true);
  }



  /**
   * Decrypts encrypted UTXO bytes with UTXO viewing key and iv from commitment.
   * @param encryptedBytes - The encrypted bytes to be decrypted.
   * @param merkleTreePdaPublicKey - The public key used in decryption.
   * @param commitment - The commitment used as the Initialization Vector (iv).
   * @returns A promise that resolves to a Result object containing the decrypted Uint8Array or an error if the decryption fails.
   */
  async decryptAesUtxo(
    encryptedBytes: Uint8Array,
    merkleTreePdaPublicKey: PublicKey,
    commitment: Uint8Array,
  ) {
    // Check if account secret key is available for decrypting using AES
    if (!this.aesSecret) {
      throw new AccountError(
        UtxoErrorCode.AES_SECRET_UNDEFINED,
        "decryptAesUtxo",
      );
    }
    setEnvironment();
    const iv16 = commitment.slice(0, 16);
    return this._decryptAes(
      encryptedBytes,
      this.getAesUtxoViewingKey(
        merkleTreePdaPublicKey,
        bs58.encode(commitment),
      ),
      iv16,
    );
  }

  /**
   * Decrypts AES encrypted bytes, the iv is expected to be the first 16 bytes.
   * @param encryptedBytes - The AES encrypted bytes to be decrypted.
   * @returns A promise that resolves to a Result containing the decrypted Uint8Array or null in case of an error.
   * @throws Will throw an error if the aesSecret is undefined.
   */
  async decryptAes(
    encryptedBytes: Uint8Array,
  ): Promise<Result<Uint8Array | null, Error>> {
    if (!this.aesSecret) {
      throw new AccountError(UtxoErrorCode.AES_SECRET_UNDEFINED, "decryptAes");
    }

    const iv16 = encryptedBytes.slice(0, 16);
    return this._decryptAes(encryptedBytes.slice(16), this.aesSecret, iv16);
  }

  /**
   * Private aes decryption method.
   * @private
   * @param encryptedBytes - The AES encrypted bytes to be decrypted.
   * @param secretKey - The secret key to be used for decryption.
   * @param iv16 - The Initialization Vector (iv) to be used for decryption.
   * @returns A promise that resolves to a Result containing the decrypted Uint8Array or null in case of an error.
   */
  private async _decryptAes(
    encryptedBytes: Uint8Array,
    secretKey: Uint8Array,
    iv16: Uint8Array,
  ): Promise<Result<Uint8Array | null, Error>> {
    try {
      return Result.Ok(
        await decrypt(encryptedBytes, secretKey, iv16, "aes-256-cbc", true),
      );
    } catch (error: any) {
      return Result.Err(error);
    }
  }

  /**
   * Encrypts utxo bytes to public key using a nonce and a standardized secret for hmac.
   * @static
   * @param publicKey - The public key to encrypt to.
   * @param bytes_message - The message to be encrypted.
   * @param commitment - The commitment used to generate the nonce.
   * @returns The encrypted Uint8Array.
   */
  static encryptNaclUtxo(
    publicKey: Uint8Array,
    messageBytes: Uint8Array,
    commitment: Uint8Array,
  ) {
    const nonce = commitment.subarray(0, 24);

    // CONSTANT_SECRET_AUTHKEY is used to minimize the number of bytes sent to the blockchain.
    // This results in poly135 being useless since the CONSTANT_SECRET_AUTHKEY is public.
    // However, ciphertext integrity is guaranteed since a hash of the ciphertext is included in a zero-knowledge proof.
    return Account.encryptNacl(
      publicKey,
      messageBytes,
      CONSTANT_SECRET_AUTHKEY,
      true,
      nonce,
      true,
    );
  }




  /**
   * Encrypts bytes to a public key.
   * @static
   * @param publicKey - The public key to encrypt to.
   * @param messageBytes - The message to be encrypted.
   * @param signerSecretKey - Optional signing secret key.
   * @param returnWithoutSigner - Optional flag to return without signer.
   * @param nonce - Optional nonce, generates random if undefined.
   * @param returnWithoutNonce - Optional flag to return without nonce.
   * @returns The encrypted Uint8Array.
   */
  static encryptNacl(
    publicKey: Uint8Array,
    messageBytes: Uint8Array,
    signerSecretKey?: Uint8Array,
    returnWithoutSigner?: boolean,
    nonce?: Uint8Array,
    returnWithoutNonce?: boolean,
  ): Uint8Array {
    if (!nonce) {
      nonce = nacl.randomBytes(nacl.nonceLength);
    }
    if (!signerSecretKey) {
      signerSecretKey = nacl.box.keyPair.generate().secretKey;
    }
    const ciphertext = box(messageBytes, nonce!, publicKey, signerSecretKey!);

    if (returnWithoutNonce) {
      return Uint8Array.from([...ciphertext]);
    }
    if (returnWithoutSigner) {
      return Uint8Array.from([...nonce!, ...ciphertext]);
    }
    return Uint8Array.from([
      ...nonce!,
      ...nacl.box.keyPair.fromSecretKey(signerSecretKey).publicKey,
      ...ciphertext,
    ]);
  }

  /**
   * Decrypts encrypted UTXO bytes.
   * @param ciphertext - The encrypted bytes to be decrypted.
   * @param commitment - The commitment used to generate the nonce.
   * @returns A promise that resolves to a Result containing the decrypted Uint8Array or null in case of an error.
   */
  async decryptNaclUtxo(
    ciphertext: Uint8Array,
    commitment: Uint8Array,
  ): Promise<Result<Uint8Array | null, Error>> {
    const nonce = commitment.slice(0, 24);
    return this._decryptNacl(
      ciphertext,
      nonce,
      nacl.box.keyPair.fromSecretKey(CONSTANT_SECRET_AUTHKEY).publicKey,
    );
  }

  /**
   * Decrypts encrypted bytes.
   * If nonce is not provided, it expects the first 24 bytes to be the nonce.
   * If signerPublicKey is not provided, expects the subsequent 32 bytes (after the nonce) to be the signer public key.
   * @param ciphertext - The encrypted bytes to be decrypted.
   * @param nonce - Optional nonce, if not provided, it is extracted from the ciphertext.
   * @param signerpublicKey - Optional signer public key, if not provided, it is extracted from the ciphertext.
   * @returns A promise that resolves to a Result containing the decrypted Uint8Array or null in case of an error.
   */
  async decryptNacl(
    ciphertext: Uint8Array,
    nonce?: Uint8Array,
    signerPublicKey?: Uint8Array,
  ): Promise<Result<Uint8Array | null, Error>> {
    if (!nonce) {
      nonce = ciphertext.slice(0, 24);
      ciphertext = ciphertext.slice(24);
    }
    if (!signerPublicKey) {
      signerPublicKey = ciphertext.slice(0, 32);
      ciphertext = ciphertext.slice(32);
    }
    return this._decryptNacl(ciphertext, nonce, signerPublicKey);
  }

  private async _decryptNacl(
    ciphertext: Uint8Array,
    nonce: Uint8Array,
    signerPublicKey?: Uint8Array,
  ): Promise<Result<Uint8Array | null, Error>> {
    return Result.Ok(
      nacl.box.open(
        ciphertext,
        nonce,
        signerPublicKey,
        this.encryptionKeypair.secretKey,
      ),
    );
  }

  private addPrivateKey(proofInput: any, transaction: TransactionParameters) {
    proofInput["inPrivateKey"] = transaction.inputUtxos.map((utxo: Utxo) => {
      if (utxo.publicKey == this.pubkey) {
        return this.privkey;
      }
      if (STANDARD_SHIELDED_PUBLIC_KEY.eq(utxo.publicKey)) {
        return STANDARD_SHIELDED_PRIVATE_KEY;
      }
    });
  }

  async getProofInternal(
    firstPath: string,
    transaction: TransactionParameters | any,
    proofInput: any,
    addPrivateKey: boolean = false,
    enableLogging: boolean = false,
  ) {
    if (!proofInput)
      throw new AccountError(
        TransactionErrorCode.PROOF_INPUT_UNDEFINED,
        "getProofInternal",
      );
    if (!transaction)
      throw new AccountError(
        TransactionErrorCode.NO_PARAMETERS_PROVIDED,
        "getProofInternal",
      );
    if (!transaction.verifierIdl)
      throw new AccountError(
        TransactionErrorCode.NO_PARAMETERS_PROVIDED,
        "getProofInternal",
        "verifierIdl is missing in TransactionParameters",
      );
    if (addPrivateKey) {
      this.addPrivateKey(proofInput, transaction);
    }
    const prover = new Prover(
      transaction.verifierIdl,
      firstPath,
      transaction.circuitName,
    );
    await prover.addProofInputs(proofInput);
    const prefix = `\x1b[37m[${new Date(Date.now()).toISOString()}]\x1b[0m`;
    const logMsg = `${prefix} Proving ${transaction.verifierIdl.name} circuit`;
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

  static createFromSeed(
    poseidon: Poseidon,
    seed: string,
    eddsa?: any,
  ): Account {
    return new Account({ poseidon, seed, eddsa });
  }

  static createFromSolanaKeypair(
    poseidon: Poseidon,
    keypair: Keypair,
    eddsa?: any,
  ): Account {
    const encodedMessage = utils.bytes.utf8.encode(SIGN_MESSAGE);
    const signature: Uint8Array = sign.detached(
      encodedMessage,
      keypair.secretKey,
    );
    return new Account({
      poseidon,
      seed: bs58.encode(signature),
      solanaPublicKey: keypair.publicKey,
      eddsa,
    });
  }

  static async createFromBrowserWallet(
    poseidon: Poseidon,
    wallet: Wallet,
    eddsa?: any,
  ): Promise<Account> {
    const encodedMessage = utils.bytes.utf8.encode(SIGN_MESSAGE);
    const signature: Uint8Array = await wallet.signMessage(encodedMessage);
    return new Account({
      poseidon,
      seed: bs58.encode(signature),
      solanaPublicKey: wallet.publicKey,
      eddsa,
    });
  }
}
