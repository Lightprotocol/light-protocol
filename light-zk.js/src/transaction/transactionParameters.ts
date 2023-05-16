import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BN, BorshAccountsCoder, Program, Idl } from "@coral-xyz/anchor";
import {
  AUTHORITY,
  MESSAGE_MERKLE_TREE_KEY,
  TRANSACTION_MERKLE_TREE_KEY,
  verifierProgramZeroProgramId,
  verifierProgramStorageProgramId,
} from "../constants";
import { N_ASSET_PUBKEYS, Utxo } from "../utxo";
import { MerkleTreeConfig } from "../merkleTree/merkleTreeConfig";
import {
  FIELD_SIZE,
  hashAndTruncateToCircuit,
  Account,
  merkleTreeProgramId,
  Relayer,
  TransactionErrorCode,
  TransactionError,
  TransactionParametersErrorCode,
  Provider,
  TransactioParametersError,
  UserErrorCode,
  RelayerErrorCode,
  CreateUtxoErrorCode,
  selectInUtxos,
  Transaction,
  Action,
  TokenData,
  transactionParameters,
  lightAccounts,
  IDL_VERIFIER_PROGRAM_ZERO,
  AppUtxoConfig,
  validateUtxoAmounts,
  createOutUtxos,
  IDL_VERIFIER_PROGRAM_ONE,
} from "../index";
import nacl from "tweetnacl";
import { sha256 } from "@noble/hashes/sha256";
import { SPL_NOOP_PROGRAM_ID } from "@solana/spl-account-compression";
const { keccak_256 } = require("@noble/hashes/sha3");

/**
 * Represents the configuration of a verifier.
 *
 * This object contains the lengths of the `inputNullifier` and `outputCommitment` fields
 * from a verifier program's IDL object. These lengths are used to verify the validity of
 * the inputs and outputs.
 *
 * @typedef {Object} VerifierConfig
 * @property {number} in - The number of nullifiers which is the length of the `inputNullifier` field in the verifier program's IDL object.
 * @property {number} out - The number of leaves which is the length of the `outputCommitment` field in the verifier program's IDL object.
 */
type VerifierConfig = {
  in: number;
  out: number;
};

/**
 * A class that represents the parameters required for a transaction.
 */
export class TransactionParameters implements transactionParameters {
  message?: Buffer;
  inputUtxos: Array<Utxo>;
  outputUtxos: Array<Utxo>;
  accounts: lightAccounts;
  // @ts-ignore:
  relayer: Relayer;
  encryptedUtxos?: Uint8Array;
  poseidon: any;
  publicAmountSpl: BN;
  publicAmountSol: BN;
  assetPubkeys: PublicKey[];
  assetPubkeysCircuit: string[];
  action: Action;
  ataCreationFee?: boolean;
  transactionNonce: number;
  txIntegrityHash?: BN;
  verifierIdl: Idl;
  verifierProgramId: PublicKey;
  verifierConfig: VerifierConfig;

  /**
   * Creates an instance of TransactionParameters.
   *
   * @param {object} options - An object containing the parameters for the transaction.
   * @param {Buffer} [options.message] - Optional message for the transaction.
   * @param {PublicKey} [options.messageMerkleTreePubkey] - Optional public key of the message Merkle tree.
   * @param {PublicKey} options.transactionMerkleTreePubkey - Public key of the transaction Merkle tree.
   * @param {PublicKey} [options.senderSpl] - Optional public key of the sender for SPL transactions.
   * @param {PublicKey} [options.recipientSpl] - Optional public key of the recipient for SPL transactions.
   * @param {PublicKey} [options.senderSol] - Optional public key of the sender for SOL transactions.
   * @param {PublicKey} [options.recipientSol] - Optional public key of the recipient for SOL transactions.
   * @param {Utxo[]} [options.inputUtxos] - Optional array of input UTXOs for the transaction.
   * @param {Utxo[]} [options.outputUtxos] - Optional array of output UTXOs for the transaction.
   * @param {Relayer} [options.relayer] - Optional relayer for the transaction.
   * @param {Uint8Array} [options.encryptedUtxos] - Optional encrypted UTXOs for the transaction.
   * @param {any} options.poseidon - Poseidon hasher for the transaction.
   * @param {Action} options.action - Action to perform in the transaction.
   * @param {PublicKey} [options.lookUpTable] - Optional lookup table for the transaction.
   * @param {Provider} [options.provider] - Optional provider for the transaction.
   * @param {boolean} [options.ataCreationFee] - Optional flag indicating whether to include the ATA creation fee in the transaction.
   * @param {number} options.transactionNonce - Nonce for the transaction.
   * @param {boolean} [options.validateUtxos] - Optional flag indicating whether to validate UTXOs in the transaction.
   * @param {Idl} options.verifierIdl - Interface description language for the transaction verifier.
   *
   * @throws {TransactioParametersError} TransactionParametersError: If no output UTXOs and input UTXOs are provided,
   * no verifier IDL is provided, no Poseidon hasher is provided, no action is defined,
   * message Merkle tree pubkey needs to be defined if a message is provided,
   * message needs to be defined if a message Merkle tree is provided, etc.
   */
  constructor({
    message,
    messageMerkleTreePubkey,
    transactionMerkleTreePubkey,
    senderSpl,
    recipientSpl,
    senderSol,
    recipientSol,
    inputUtxos,
    outputUtxos,
    relayer,
    encryptedUtxos,
    poseidon,
    action,
    lookUpTable,
    ataCreationFee,
    transactionNonce,
    validateUtxos = true,
    verifierIdl,
  }: {
    message?: Buffer;
    messageMerkleTreePubkey?: PublicKey;
    transactionMerkleTreePubkey: PublicKey;
    senderSpl?: PublicKey;
    recipientSpl?: PublicKey;
    senderSol?: PublicKey;
    recipientSol?: PublicKey;
    inputUtxos?: Utxo[];
    outputUtxos?: Utxo[];
    relayer?: Relayer;
    encryptedUtxos?: Uint8Array;
    poseidon: any;
    action: Action;
    lookUpTable?: PublicKey;
    provider?: Provider;
    ataCreationFee?: boolean;
    transactionNonce: number;
    validateUtxos?: boolean;
    verifierIdl: Idl;
  }) {
    if (!outputUtxos && !inputUtxos) {
      throw new TransactioParametersError(
        TransactionErrorCode.NO_UTXOS_PROVIDED,
        "constructor",
        "",
      );
    }

    if (!verifierIdl) {
      throw new TransactioParametersError(
        TransactionParametersErrorCode.NO_VERIFIER_IDL_PROVIDED,
        "constructor",
        "",
      );
    }

    if (!poseidon) {
      throw new TransactioParametersError(
        TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
        "constructor",
        "",
      );
    }

    if (!action) {
      throw new TransactioParametersError(
        TransactionParametersErrorCode.NO_ACTION_PROVIDED,
        "constructor",
        "Define an action either Action.TRANSFER, Action.SHIELD,Action.UNSHIELD",
      );
    }

    if (message && !messageMerkleTreePubkey) {
      throw new TransactioParametersError(
        TransactionParametersErrorCode.MESSAGE_MERKLE_TREE_UNDEFINED,
        "constructor",
        "Message Merkle tree pubkey needs to be defined if you provide a message",
      );
    }
    if (messageMerkleTreePubkey && !message) {
      throw new TransactioParametersError(
        TransactionParametersErrorCode.MESSAGE_UNDEFINED,
        "constructor",
        "Message needs to be defined if you provide message Merkle tree",
      );
    }
    this.verifierProgramId =
      TransactionParameters.getVerifierProgramId(verifierIdl);
    this.verifierConfig = TransactionParameters.getVerifierConfig(verifierIdl);
    this.transactionNonce = transactionNonce;
    this.message = message;
    this.verifierIdl = verifierIdl;
    this.poseidon = poseidon;
    this.ataCreationFee = ataCreationFee;
    this.encryptedUtxos = encryptedUtxos;
    this.action = action;
    this.inputUtxos = this.addEmptyUtxos(inputUtxos, this.verifierConfig.in);
    this.outputUtxos = this.addEmptyUtxos(outputUtxos, this.verifierConfig.out);
    if (action === Action.SHIELD && senderSol && lookUpTable) {
      this.relayer = new Relayer(senderSol, lookUpTable);
    } else if (action === Action.SHIELD && !senderSol) {
      throw new TransactioParametersError(
        TransactionErrorCode.SOL_SENDER_UNDEFINED,
        "constructor",
        "Sender sol always needs to be defined because we use it as the signer to instantiate the relayer object.",
      );
    } else if (action === Action.SHIELD && !lookUpTable) {
      throw new TransactioParametersError(
        TransactionParametersErrorCode.LOOK_UP_TABLE_UNDEFINED,
        "constructor",
        "At deposit lookup table needs to be defined to instantiate a relayer object with yourself as the relayer.",
      );
    }

    if (action !== Action.SHIELD) {
      if (relayer) {
        this.relayer = relayer;
      } else {
        throw new TransactioParametersError(
          TransactionErrorCode.RELAYER_UNDEFINED,
          "constructor",
          "For a transfer or withdrawal a relayer needs to be provided.",
        );
      }
    }

    const pubkeys = TransactionParameters.getAssetPubkeys(
      this.inputUtxos,
      this.outputUtxos,
    );

    this.assetPubkeys = pubkeys.assetPubkeys;
    this.assetPubkeysCircuit = pubkeys.assetPubkeysCircuit;
    this.publicAmountSol = TransactionParameters.getExternalAmount(
      0,
      this.inputUtxos,
      this.outputUtxos,
      this.assetPubkeysCircuit,
    );
    this.publicAmountSpl = TransactionParameters.getExternalAmount(
      1,
      this.inputUtxos,
      this.outputUtxos,
      this.assetPubkeysCircuit,
    );
    // safeguard should not be possible
    if (!this.publicAmountSol.gte(new BN(0)))
      throw new TransactioParametersError(
        TransactionParametersErrorCode.PUBLIC_AMOUNT_NEGATIVE,
        "constructor",
        "Public sol amount cannot be negative.",
      );
    if (!this.publicAmountSpl.gte(new BN(0)))
      throw new TransactioParametersError(
        TransactionParametersErrorCode.PUBLIC_AMOUNT_NEGATIVE,
        "constructor",
        "Public spl amount cannot be negative.",
      );

    // Checking plausibility of inputs
    if (this.action === Action.SHIELD) {
      /**
       * No relayer
       * public amounts are u64s
       * senderSpl is the user
       * recipientSpl is the merkle tree
       */
      if (relayer)
        throw new TransactioParametersError(
          TransactionParametersErrorCode.RELAYER_DEFINED,
          "constructor",
          "For a deposit no relayer should to be provided, the user send the transaction herself.",
        );
      try {
        this.publicAmountSol.toArray("be", 8);
      } catch (error) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          "constructor",
          `Public amount sol ${this.publicAmountSol} needs to be a u64 at deposit. Check whether you defined input and output utxos correctly, for a deposit the amounts of output utxos need to be bigger than the amounts of input utxos`,
        );
      }

      try {
        this.publicAmountSpl.toArray("be", 8);
      } catch (error) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          "constructor",
          `Public amount spl ${this.publicAmountSpl} needs to be a u64 at deposit. Check whether you defined input and output utxos correctly, for a deposit the amounts of output utxos need to be bigger than the amounts of input utxos`,
        );
      }
      if (!this.publicAmountSol.eq(new BN(0)) && recipientSol) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          "constructor",
          "",
        );
      }
      if (!this.publicAmountSpl.eq(new BN(0)) && recipientSpl) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          "constructor",
          "",
        );
      }

      if (!this.publicAmountSol.eq(new BN(0)) && !senderSol) {
        throw new TransactioParametersError(
          TransactionErrorCode.SOL_SENDER_UNDEFINED,
          "constructor",
          "",
        );
      }
      if (!this.publicAmountSpl.eq(new BN(0)) && !senderSpl) {
        throw new TransactioParametersError(
          TransactionErrorCode.SPL_SENDER_UNDEFINED,
          "constructor",
          "",
        );
      }
    } else if (this.action === Action.UNSHIELD) {
      /**
       * relayer is defined
       * public amounts sub FieldSize are negative or 0
       * for public amounts greater than 0 a recipientSpl needs to be defined
       * senderSpl is the merkle tree
       * recipientSpl is the user
       */
      // TODO: should I throw an error when a lookup table is defined?
      if (!relayer)
        throw new TransactioParametersError(
          TransactionErrorCode.RELAYER_UNDEFINED,
          "constructor",
          "For a withdrawal a relayer needs to be provided.",
        );
      // public amount is either 0 or negative
      // this.publicAmountSol.add(FIELD_SIZE).mod(FIELD_SIZE) this changes the value
      const tmpSol = this.publicAmountSol;
      if (!tmpSol.sub(FIELD_SIZE).lte(new BN(0)))
        throw new TransactioParametersError(
          TransactionParametersErrorCode.INVALID_PUBLIC_AMOUNT,
          "constructor",
          "",
        );
      const tmpSpl = this.publicAmountSpl;
      if (!tmpSpl.sub(FIELD_SIZE).lte(new BN(0)))
        throw new TransactioParametersError(
          TransactionParametersErrorCode.INVALID_PUBLIC_AMOUNT,
          "constructor",
          "",
        );
      try {
        if (!tmpSol.eq(new BN(0))) {
          tmpSol.sub(FIELD_SIZE).toArray("be", 8);
        }
      } catch (error) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          "constructor",
          "Public amount needs to be a u64 at deposit.",
        );
      }

      try {
        if (!tmpSpl.eq(new BN(0))) {
          tmpSpl.sub(FIELD_SIZE).toArray("be", 8);
        }
      } catch (error) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          "constructor",
          "Public amount needs to be a u64 at deposit.",
        );
      }

      if (!this.publicAmountSol.eq(new BN(0)) && !recipientSol) {
        throw new TransactioParametersError(
          TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
          "constructor",
          "",
        );
      }

      if (!this.publicAmountSpl.eq(new BN(0)) && !recipientSpl) {
        throw new TransactioParametersError(
          TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
          "constructor",
          "",
        );
      }
      // && senderSol.toBase58() != merkle tree token pda
      if (!this.publicAmountSol.eq(new BN(0)) && senderSol) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          "constructor",
          "",
        );
      }
      if (!this.publicAmountSpl.eq(new BN(0)) && senderSpl) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SPL_SENDER_DEFINED,
          "constructor",
          "",
        );
      }
    } else if (this.action === Action.TRANSFER) {
      /**
       * relayer is defined
       * public amount spl amount is 0
       * public amount spl amount sub FieldSize is equal to the relayer fee
       * senderSpl is the merkle tree
       * recipientSpl does not exists it is an internal transfer just the relayer is paid
       */
      if (!relayer)
        throw new TransactioParametersError(
          TransactionErrorCode.RELAYER_UNDEFINED,
          "constructor",
          "For a transfer a relayer needs to be provided.",
        );
      if (!this.publicAmountSpl.eq(new BN(0)))
        throw new TransactioParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_SPL_NOT_ZERO,
          "constructor",
          "For a transfer public spl amount needs to be zero",
        );

      const tmpSol = this.publicAmountSol;

      if (
        !tmpSol
          .sub(FIELD_SIZE)
          .mul(new BN(-1))
          .eq(relayer.getRelayerFee(ataCreationFee))
      )
        throw new TransactioParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_SOL_NOT_ZERO,
          "constructor",
          `public amount ${tmpSol
            .sub(FIELD_SIZE)
            .mul(new BN(-1))}  should be ${relayer.getRelayerFee(
            ataCreationFee,
          )}`,
        );

      if (recipientSpl) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          "constructor",
          "This is a transfer, no spl amount should be withdrawn. To withdraw an spl amount mark the transaction as withdrawal.",
        );
      }

      if (recipientSol) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          "constructor",
          "This is a transfer, no sol amount should be withdrawn. To withdraw an sol amount mark the transaction as withdrawal.",
        );
      }

      if (senderSol) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          "constructor",
          "",
        );
      }
      if (senderSpl) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SPL_SENDER_DEFINED,
          "constructor",
          "",
        );
      }
    } else {
      throw new TransactioParametersError(
        TransactionParametersErrorCode.NO_ACTION_PROVIDED,
        "constructor",
        "",
      );
    }

    this.accounts = {
      systemProgramId: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      logWrapper: SPL_NOOP_PROGRAM_ID,
      messageMerkleTree: messageMerkleTreePubkey,
      transactionMerkleTree: transactionMerkleTreePubkey,
      registeredVerifierPda: Transaction.getRegisteredVerifierPda(
        merkleTreeProgramId,
        this.verifierProgramId,
      ),
      authority: Transaction.getSignerAuthorityPda(
        merkleTreeProgramId,
        this.verifierProgramId,
      ),
      senderSpl: senderSpl,
      recipientSpl: recipientSpl,
      senderSol: senderSol, // TODO: change to senderSol
      recipientSol: recipientSol, // TODO: change name to recipientSol
      programMerkleTree: merkleTreeProgramId,
      tokenAuthority: Transaction.getTokenAuthority(),
      verifierProgram: this.verifierProgramId,
    };

    this.assignAccounts();
    // @ts-ignore:
    this.accounts.signingAddress = this.relayer.accounts.relayerPubkey;
  }

  /**
   * Asynchronously converts transaction parameters to bytes using the BorshAccountsCoder.
   *
   * This method performs the following operations:
   * 1. Initializes a new BorshAccountsCoder with the IDL_VERIFIER_PROGRAM_ZERO.
   * 2. Converts each utxo in the `inputUtxos` array to bytes and stores them in `inputUtxosBytes`.
   * 3. Converts each utxo in the `outputUtxos` array to bytes and stores them in `outputUtxosBytes`.
   * 4. Prepares an object containing the `outputUtxosBytes`, `inputUtxosBytes`, `relayerPubkey`, `relayerFee`, current object's properties, accounts' properties, and `transactionNonce` (converted to a BN instance).
   * 5. Encodes the prepared object under the "transactionParameters" key using the BorshAccountsCoder.
   *
   * @returns {Promise<Buffer>} A promise that resolves to a Buffer containing the encoded transaction parameters.
   *
   * @throws {Error} Throws an error if the encoding fails.
   */
  async toBytes(): Promise<Buffer> {
    let coder = new BorshAccountsCoder(IDL_VERIFIER_PROGRAM_ZERO);
    let inputUtxosBytes: any[] = [];
    for (var utxo of this.inputUtxos) {
      inputUtxosBytes.push(await utxo.toBytes());
    }
    let outputUtxosBytes: any[] = [];
    for (var utxo of this.outputUtxos) {
      outputUtxosBytes.push(await utxo.toBytes());
    }
    let preparedObject = {
      outputUtxosBytes,
      inputUtxosBytes,
      relayerPubkey: this.relayer.accounts.relayerPubkey,
      relayerFee: this.relayer.relayerFee,
      ...this,
      ...this.accounts,
      transactionNonce: new BN(this.transactionNonce),
    };
    return await coder.encode("transactionParameters", preparedObject);
  }

  /**
   * A static method to find the index of a specific IDL object in an array of IDL objects based on a given program ID.
   *
   * @param {string} programId - The ID of the program for which to find the IDL object.
   * @param {anchor.Idl[]} idlObjects - An array of IDL objects among which to search.
   * @returns {number} The index of the IDL object that contains the provided program ID. Returns -1 if the program ID is not found.
   * @throws {Error} If an IDL object in the provided array does not have any constants.
   *
   * @example
   * ```typescript
   * let index = TransactionParameters.findIdlIndex("someProgramId", idlArray);
   * if(index !== -1) {
   *   console.log("Program ID found at index: " + index);
   * }
   * ```
   */
  static findIdlIndex(programId: string, idlObjects: anchor.Idl[]): number {
    for (let i = 0; i < idlObjects.length; i++) {
      const constants = idlObjects[i].constants;
      if (!constants)
        throw new Error(`Idl in index ${i} does not have any constants`);

      for (const constant of constants) {
        if (
          constant.name === "PROGRAM_ID" &&
          constant.type === "string" &&
          constant.value === `"${programId}"`
        ) {
          return i;
        }
      }
    }

    return -1; // Return -1 if the programId is not found in any IDL object
  }

  /**
   * A static method to retrieve the verifier program ID from a given IDL object.
   *
   * @static
   * @param {Idl} verifierIdl - The IDL object containing the verifier program ID.
   * @returns {PublicKey} The verifier program ID as a PublicKey object.
   *
   * @example
   * ```typescript
   * let verifierProgramId = TransactionParameters.getVerifierProgramId(verifierIdl);
   * console.log("Verifier program ID: " + verifierProgramId);
   * ```
   * @remarks
   * The programID is expected to be appended as a constant to the program that can be read directly from the IDL.
   */
  static getVerifierProgramId(verifierIdl: Idl): PublicKey {
    const programId = new PublicKey(
      verifierIdl.constants![0].value.slice(1, -1),
    );
    return programId;
  }

  /**
   * A static method to instantiate a new verifier program from a given IDL object.
   *
   * @static
   * @param {Idl} verifierIdl - The IDL object of the verifier program.
   * @returns {Program<Idl>} A new Anchor Program object for the verifier program.
   * @remarks
   * The programID is expected to be appended as a constant to the program that can be read directly from the IDL.
   */
  static getVerifierProgram(verifierIdl: Idl): Program<Idl> {
    const programId = new PublicKey(
      verifierIdl.constants![0].value.slice(1, -1),
    );
    const verifierProgram = new Program(verifierIdl, programId);
    return verifierProgram;
  }

  /**
   * A static method to fetch the verifier configuration from a given IDL object.
   *
   * This method parses the IDL object to identify an account with a name
   * that starts with "zK" and ends with "ProofInputs". It then examines the fields
   * of this account to identify the `inputNullifier` and `outputCommitment` fields,
   * checking that they are of the correct array type, and retrieves their lengths.
   * The lengths of these fields are then returned as a VerifierConfig object.
   *
   * @static
   * @param {Idl} verifierIdl - The IDL object of the verifier program.
   * @returns {VerifierConfig} A VerifierConfig object with the lengths of the `inputNullifier` and `outputCommitment` fields.
   *
   * @throws {Error} Throws an error if no matching account is found in the IDL, or if the `inputNullifier` or `outputCommitment` fields are not found or are of incorrect type.
   *
   */
  static getVerifierConfig(verifierIdl: Idl): VerifierConfig {
    const accounts = verifierIdl.accounts;
    const resultElement = accounts!.find(
      (account) =>
        account.name.startsWith("zK") && account.name.endsWith("ProofInputs"),
    );

    if (!resultElement) {
      throw new Error("No matching element found");
    }
    interface Field {
      name: string;
      type: any;
    }

    const fields = resultElement.type.fields;
    const inputNullifierField = fields.find(
      (field) => field.name === "inputNullifier",
    ) as Field;
    const outputCommitmentField = fields.find(
      (field) => field.name === "outputCommitment",
    ) as Field;

    if (!inputNullifierField || !inputNullifierField.type.array) {
      throw new Error(
        "inputNullifier field not found or has an incorrect type",
      );
    }

    if (!outputCommitmentField || !outputCommitmentField.type.array) {
      throw new Error(
        "outputCommitment field not found or has an incorrect type",
      );
    }

    const inputNullifierLength = inputNullifierField.type.array[1];
    const outputCommitmentLength = outputCommitmentField.type.array[1];

    return { in: inputNullifierLength, out: outputCommitmentLength };
  }

  /**
   * A static method to create a new TransactionParameters instance from a given set of bytes.
   *
   * This method decodes the provided bytes using a BorshAccountsCoder and checks the validity of the resulting data.
   * It retrieves the input and output UTXOs from the decoded data, ensuring that they match the provided IDLs.
   * It also checks that the relayer's public key matches the one in the decoded data.
   *
   * If the decoded recipient is not the AUTHORITY, the action is set to UNSHIELD, otherwise, it's set to TRANSFER.
   * The method then creates a new TransactionParameters instance with the retrieved data and returns it.
   *
   * @static
   * @param {Object} params - The parameters for the method.
   * @param {any} params.poseidon - The Poseidon hash function instance.
   * @param {anchor.Idl[]} [params.utxoIdls] - An optional array of IDLs for the UTXOs.
   * @param {Buffer} params.bytes - The bytes to decode into a TransactionParameters instance.
   * @param {Relayer} params.relayer - The relayer for the transaction.
   * @param {Idl} params.verifierIdl - The IDL of the verifier program.
   * @returns {Promise<TransactionParameters>} A promise that resolves to a new TransactionParameters instance.
   *
   * @throws {TransactioParametersError} Throws a TransactionParametersError if the UTXO IDLs are not provided when needed, or if the relayer's public key does not match the one in the decoded data.
   *
   */
  static async fromBytes({
    poseidon,
    utxoIdls,
    bytes,
    relayer,
    verifierIdl,
    assetLookupTable,
    verifierProgramLookupTable,
  }: {
    poseidon: any;
    utxoIdls?: anchor.Idl[];
    bytes: Buffer;
    relayer: Relayer;
    verifierIdl: Idl;
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
  }): Promise<TransactionParameters> {
    let coder = new BorshAccountsCoder(IDL_VERIFIER_PROGRAM_ZERO);
    let decoded = coder.decodeUnchecked("transactionParameters", bytes);

    const getUtxos = (
      utxoBytesArray: Array<Buffer>,
      utxoIdls?: anchor.Idl[],
    ) => {
      let utxos: Utxo[] = [];
      for (var [_, utxoBytes] of utxoBytesArray.entries()) {
        let appDataIdl: any = undefined;
        if (
          utxoBytes.subarray(128, 160).toString() !==
          Buffer.alloc(32).fill(0).toString()
        ) {
          if (!utxoIdls) {
            throw new TransactioParametersError(
              TransactionParametersErrorCode.UTXO_IDLS_UNDEFINED,
              "fromBytes",
            );
          }
          let idlIndex = TransactionParameters.findIdlIndex(
            new PublicKey(utxoBytes.subarray(128, 160)).toBase58(),
            utxoIdls,
          );
          // could add option to fetch idl from chain if not found
          appDataIdl = utxoIdls[idlIndex];
        }
        utxos.push(
          Utxo.fromBytes({
            poseidon,
            bytes: utxoBytes,
            appDataIdl,
            assetLookupTable,
            verifierProgramLookupTable,
          }),
        );
      }
      return utxos;
    };

    const inputUtxos = getUtxos(decoded.inputUtxosBytes, utxoIdls);
    const outputUtxos = getUtxos(decoded.outputUtxosBytes, utxoIdls);

    if (
      relayer &&
      relayer.accounts.relayerPubkey.toBase58() != decoded.relayerPubkey
    ) {
      // TODO: add functionality to look up relayer or fetch info, looking up is better
      throw new TransactioParametersError(
        TransactionParametersErrorCode.RELAYER_INVALID,
        "fromBytes",
        "The provided relayer has a different public key as the relayer publickey decoded from bytes",
      );
    }
    if (!relayer) {
      throw new TransactioParametersError(
        TransactionErrorCode.RELAYER_UNDEFINED,
        "fromBytes",
      );
    }

    let action = Action.TRANSFER;
    if (
      decoded.recipientSol.toBase58() !== AUTHORITY.toBase58() ||
      decoded.recipientSpl.toBase58() !== AUTHORITY.toBase58()
    ) {
      action = Action.UNSHIELD;
    } else {
      decoded.recipientSol = undefined;
      decoded.recipientSpl = undefined;
    }
    return new TransactionParameters({
      poseidon,
      inputUtxos,
      outputUtxos,
      relayer,
      ...decoded,
      action,
      transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
      verifierIdl: verifierIdl,
    });
  }

  /**
   * Static async method to generate transaction parameters.
   *
   * @static
   * @async
   * @param {Object} params - Parameters for generating transaction parameters.
   * @param {TokenData} params.tokenCtx - The context of the token involved in the transaction.
   * @param {BN} params.publicAmountSpl - The amount of SPL tokens involved in the transaction.
   * @param {BN} params.publicAmountSol - The amount of SOL tokens involved in the transaction.
   * @param {PublicKey} params.userSplAccount - The SPL account of the user.
   * @param {Account} params.account - The account involved in the transaction.
   * @param {Utxo[]} params.utxos - Array of UTXO (Unspent Transaction Outputs) objects.
   * @param {PublicKey} params.recipientSol - The Solana address of the recipient.
   * @param {PublicKey} params.recipientSplAddress - The SPL address of the recipient.
   * @param {Utxo[]} params.inUtxos - Array of input UTXOs for the transaction.
   * @param {Utxo[]} params.outUtxos - Array of output UTXOs for the transaction.
   * @param {Action} params.action - The action being performed (shield, unshield, transfer).
   * @param {Provider} params.provider - The provider for the transaction.
   * @param {Relayer} params.relayer - The relayer for the transaction.
   * @param {boolean} params.ataCreationFee - Whether to include the ATA (Associated Token Account) creation fee.
   * @param {number} params.transactionNonce - The nonce for the transaction.
   * @param {AppUtxoConfig} params.appUtxo - The configuration for the application UTXO.
   * @param {boolean} params.addInUtxos - Whether to add input UTXOs to the transaction.
   * @param {boolean} params.addOutUtxos - Whether to add output UTXOs to the transaction.
   * @param {Idl} params.verifierIdl - The IDL (Interface Description Language) for the verifier program.
   * @param {boolean} params.mergeUtxos - Whether to merge UTXOs in the transaction.
   * @param {Buffer} params.message - The message data for the transaction.
   * @returns {Promise<TransactionParameters>} - A Promise that resolves with the generated TransactionParameters object.
   *
   * @throws {TransactioParametersError} - A TransactionParametersError if action is TRANSFER and no outUtxos and mergeUtxos is not set, or if the action is not SHIELD and relayer fee is undefined, or if the account is undefined.
   * @throws {CreateUtxoErrorCode} - A CreateUtxoErrorCode if the account is undefined.
   *
   * @remarks
   * The method constructs a TransactionParameters object which includes all the necessary parameters for a transaction.
   * It selects the necessary input UTXOs, creates the output UTXOs, and validates the relayer and action of the transaction.
   * It also handles various transaction actions like shield, unshield and transfer.
   */
  static async getTxParams({
    tokenCtx,
    publicAmountSpl,
    publicAmountSol,
    action,
    userSplAccount = AUTHORITY,
    account,
    utxos,
    inUtxos,
    // for unshield
    recipientSol,
    recipientSplAddress,
    // for transfer
    outUtxos,
    relayer,
    provider,
    ataCreationFee, // associatedTokenAccount = ata
    transactionNonce,
    appUtxo,
    addInUtxos = true,
    addOutUtxos = true,
    verifierIdl,
    mergeUtxos = false,
    message,
    assetLookupTable,
    verifierProgramLookupTable,
  }: {
    tokenCtx: TokenData;
    publicAmountSpl?: BN;
    publicAmountSol?: BN;
    userSplAccount?: PublicKey;
    account: Account;
    utxos?: Utxo[];
    recipientSol?: PublicKey;
    recipientSplAddress?: PublicKey;
    inUtxos?: Utxo[];
    outUtxos?: Utxo[];
    action: Action;
    provider: Provider;
    relayer?: Relayer;
    ataCreationFee?: boolean;
    transactionNonce: number;
    appUtxo?: AppUtxoConfig;
    addInUtxos?: boolean;
    addOutUtxos?: boolean;
    verifierIdl: Idl;
    mergeUtxos?: boolean;
    message?: Buffer;
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
  }): Promise<TransactionParameters> {
    publicAmountSol = publicAmountSol ? publicAmountSol : new BN(0);
    publicAmountSpl = publicAmountSpl ? publicAmountSpl : new BN(0);

    if (action === Action.TRANSFER && !outUtxos && !mergeUtxos)
      throw new TransactioParametersError(
        UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED,
        "getTxParams",
        "Recipient outUtxo not provided for transfer",
      );

    if (action !== Action.SHIELD && !relayer?.getRelayerFee(ataCreationFee)) {
      // TODO: could make easier to read by adding separate if/cases
      throw new TransactioParametersError(
        RelayerErrorCode.RELAYER_FEE_UNDEFINED,
        "getTxParams",
        `No relayerFee provided for ${action.toLowerCase()}}`,
      );
    }
    if (!account) {
      throw new TransactioParametersError(
        CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
        "getTxParams",
        "account for change utxo is undefined",
      );
    }

    var inputUtxos: Utxo[] = inUtxos ? [...inUtxos] : [];
    var outputUtxos: Utxo[] = outUtxos ? [...outUtxos] : [];

    if (addInUtxos) {
      inputUtxos = selectInUtxos({
        publicMint: tokenCtx.mint,
        publicAmountSpl,
        publicAmountSol,
        poseidon: provider.poseidon,
        inUtxos,
        outUtxos,
        utxos,
        relayerFee: relayer?.getRelayerFee(ataCreationFee),
        action,
        numberMaxInUtxos:
          TransactionParameters.getVerifierConfig(verifierIdl).in,
      });
    }
    if (addOutUtxos) {
      outputUtxos = createOutUtxos({
        publicMint: tokenCtx.mint,
        publicAmountSpl,
        inUtxos: inputUtxos,
        publicAmountSol, // TODO: add support for extra sol for unshield & transfer
        poseidon: provider.poseidon,
        relayerFee: relayer?.getRelayerFee(ataCreationFee),
        changeUtxoAccount: account,
        outUtxos,
        action,
        appUtxo,
        numberMaxOutUtxos:
          TransactionParameters.getVerifierConfig(verifierIdl).out,
        assetLookupTable,
        verifierProgramLookupTable,
      });
    }

    let txParams = new TransactionParameters({
      outputUtxos,
      inputUtxos,
      transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
      senderSpl: action === Action.SHIELD ? userSplAccount : undefined,
      senderSol:
        action === Action.SHIELD ? provider.wallet!.publicKey : undefined,
      recipientSpl: recipientSplAddress,
      recipientSol,
      poseidon: provider.poseidon,
      action,
      lookUpTable: provider.lookUpTable!,
      relayer: relayer,
      ataCreationFee,
      transactionNonce,
      verifierIdl,
      message,
      messageMerkleTreePubkey: message ? MESSAGE_MERKLE_TREE_KEY : undefined,
    });

    return txParams;
  }

  /**
   * Adds empty UTXOs to the given array until the array reaches a specified length.
   *
   * The zero-knowledge proof circuit requires all inputs to be defined, hence the need
   * to populate the array with empty UTXOs when necessary. This function ensures that
   * the number of UTXOs in the array matches the expected number as defined by the zk-SNARKs
   * protocol.
   *
   * @param utxos - The array of UTXOs to which empty UTXOs will be added. Default is an empty array.
   * @param len - The desired number of UTXOs in the array after the function is executed.
   *
   * @returns An array of UTXOs of the desired length, populated with empty UTXOs as needed.
   */
  addEmptyUtxos(utxos: Utxo[] = [], len: number): Utxo[] {
    while (utxos.length < len) {
      utxos.push(
        new Utxo({
          poseidon: this.poseidon,
          assetLookupTable: [SystemProgram.programId.toBase58()],
          verifierProgramLookupTable: [SystemProgram.programId.toBase58()],
        }),
      );
    }
    return utxos;
  }

  /**
   * This method assigns sender and recipient accounts for Solana and SPL tokens to the transaction parameters
   * based on the action (either 'unshield', 'transfer', or 'shield').
   *
   * For 'unshield' and 'transfer' actions, it assigns the sender accounts for both SPL and Solana tokens
   * and checks if the recipient accounts are defined. If not, it throws an error.
   *
   * For the 'shield' action, it assigns the recipient accounts and checks if the sender accounts are defined.
   * If not, it throws an error.
   *
   * @throws {TransactioParametersError}
   * TransactionParametersError:
   *
   * - If the action is 'unshield' or 'transfer' and the recipient accounts for SPL or Solana tokens are undefined.
   *
   * - If the action is 'shield' and the sender accounts for SPL or Solana tokens are undefined.
   *
   * - If the assetPubkeys are undefined.
   *
   * - If the action is not 'deposit' but should be, based on the provided sender and recipient accounts and relayer.
   *
   * @returns {void}
   */
  assignAccounts() {
    if (!this.assetPubkeys)
      throw new TransactioParametersError(
        TransactionErrorCode.ASSET_PUBKEYS_UNDEFINED,
        "assignAccounts assetPubkeys undefined",
        "assignAccounts",
      );

    if (
      this.action.toString() === Action.UNSHIELD.toString() ||
      this.action.toString() === Action.TRANSFER.toString()
    ) {
      this.accounts.senderSpl = MerkleTreeConfig.getSplPoolPdaToken(
        this.assetPubkeys[1],
        merkleTreeProgramId,
      );
      this.accounts.senderSol =
        MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;

      if (!this.accounts.recipientSpl) {
        // AUTHORITY is used as place holder
        this.accounts.recipientSpl = AUTHORITY;
        if (!this.publicAmountSpl?.eq(new BN(0))) {
          throw new TransactionError(
            TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
            "assignAccounts",
            "Spl recipientSpl is undefined while public spl amount is != 0.",
          );
        }
      }

      if (!this.accounts.recipientSol) {
        // AUTHORITY is used as place holder
        this.accounts.recipientSol = AUTHORITY;
        if (
          !this.publicAmountSol.eq(new BN(0)) &&
          !this.publicAmountSol
            ?.sub(FIELD_SIZE)
            .mul(new BN(-1))
            .sub(new BN(this.relayer.getRelayerFee(this.ataCreationFee)))
            .eq(new BN(0))
        ) {
          throw new TransactioParametersError(
            TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
            "assignAccounts",
            "Sol recipientSpl is undefined while public spl amount is != 0.",
          );
        }
      }
    } else {
      if (this.action.toString() !== Action.SHIELD.toString()) {
        throw new TransactioParametersError(
          TransactionErrorCode.ACTION_IS_NO_DEPOSIT,
          "assignAccounts",
          "Action is withdrawal but should not be. Spl & sol senderSpl accounts are provided and a relayer which is used to identify transfers and withdrawals. For a deposit do not provide a relayer.",
        );
      }

      this.accounts.recipientSpl = MerkleTreeConfig.getSplPoolPdaToken(
        this.assetPubkeys[1],
        merkleTreeProgramId,
      );
      this.accounts.recipientSol =
        MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;

      if (!this.accounts.senderSpl) {
        /// assigning a placeholder account
        this.accounts.senderSpl = AUTHORITY;
        if (!this.publicAmountSpl?.eq(new BN(0))) {
          throw new TransactioParametersError(
            TransactionErrorCode.SPL_SENDER_UNDEFINED,
            "assignAccounts",
            "Spl senderSpl is undefined while public spl amount is != 0.",
          );
        }
      }
      this.accounts.senderSol = TransactionParameters.getEscrowPda(
        this.verifierProgramId,
      );
    }
  }

  /**
   * This method generates a Program Derived Address (PDA) with the seed "escrow" for the verifier program.
   * PDAs in Solana are addresses that are based off of the public key of a deployed program and are
   * unique to each specific program and seed. This method is used to get the PDA that is used as the escrow account.
   *
   * @param {PublicKey} verifierProgramId - The public key of the verifier program.
   * @returns {PublicKey} - The public key of the escrow Program Derived Address.
   */
  static getEscrowPda(verifierProgramId: PublicKey): PublicKey {
    return PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("escrow")],
      verifierProgramId,
    )[0];
  }

  /**
   * This method collects and returns public keys of assets involved in a transaction, from both input and output UTXOs.
   * It also checks whether the number of different assets involved in the transaction exceeds the maximum limit.
   * If there are less assets than the maximum allowed, it fills up the remaining space with the System Program's public key.
   *
   * @param {Utxo[]} inputUtxos - The input UTXOs for the transaction.
   * @param {Utxo[]} outputUtxos - The output UTXOs for the transaction.
   * @returns {{assetPubkeysCircuit: string[]; assetPubkeys: PublicKey[]}} - An object containing arrays of circuit and regular public keys of assets.
   *
   * @throws {TransactionError} - TransactionError: If no UTXOs are provided or if the number of different assets exceeds the maximum allowed.
   */
  static getAssetPubkeys(
    inputUtxos?: Utxo[],
    outputUtxos?: Utxo[],
  ): { assetPubkeysCircuit: string[]; assetPubkeys: PublicKey[] } {
    let assetPubkeysCircuit: string[] = [
      hashAndTruncateToCircuit(SystemProgram.programId.toBytes()).toString(),
    ];

    let assetPubkeys: PublicKey[] = [SystemProgram.programId];

    if (inputUtxos) {
      inputUtxos.map((utxo) => {
        let found = false;
        if (
          assetPubkeysCircuit.indexOf(utxo.assetsCircuit[1].toString()) !== -1
        ) {
          found = true;
        }

        if (!found && utxo.assetsCircuit[1].toString() != "0") {
          assetPubkeysCircuit.push(utxo.assetsCircuit[1].toString());
          assetPubkeys.push(utxo.assets[1]);
        }
      });
    }

    if (outputUtxos) {
      outputUtxos.map((utxo) => {
        let found = false;
        for (var i in assetPubkeysCircuit) {
          if (
            assetPubkeysCircuit.indexOf(utxo.assetsCircuit[1].toString()) !== -1
          ) {
            found = true;
          }
        }
        if (!found && utxo.assetsCircuit[1].toString() != "0") {
          assetPubkeysCircuit.push(utxo.assetsCircuit[1].toString());
          assetPubkeys.push(utxo.assets[1]);
        }
      });
    }

    if (
      (!inputUtxos && !outputUtxos) ||
      (inputUtxos?.length == 0 && outputUtxos?.length == 0)
    ) {
      throw new TransactionError(
        TransactionErrorCode.NO_UTXOS_PROVIDED,
        "getAssetPubkeys",
        "No input or output utxos provided.",
      );
    }

    // TODO: test this better
    // if (assetPubkeys.length > params?.verifier.config.out) {
    //   throw new TransactionError(
    //     TransactionErrorCode.EXCEEDED_MAX_ASSETS,
    //     "getAssetPubkeys",
    //     `Utxos contain too many different assets ${params?.verifier.config.out} > max allowed: ${N_ASSET_PUBKEYS}`,
    //   );
    // }

    if (assetPubkeys.length > N_ASSET_PUBKEYS) {
      throw new TransactionError(
        TransactionErrorCode.EXCEEDED_MAX_ASSETS,
        "getAssetPubkeys",
        `Utxos contain too many different assets ${assetPubkeys.length} > max allowed: ${N_ASSET_PUBKEYS}`,
      );
    }

    while (assetPubkeysCircuit.length < N_ASSET_PUBKEYS) {
      assetPubkeysCircuit.push(new BN(0).toString());
      assetPubkeys.push(SystemProgram.programId);
    }

    return { assetPubkeysCircuit, assetPubkeys };
  }

  /**
   * This method calculates the external amount for a specified asset.
   * It achieves this by adding all output UTXOs of the same asset and subtracting all input UTXOs of the same asset.
   * The result is then added to the field size and the modulus of the field size is returned.
   *
   * @param {number} assetIndex - The index of the asset for which the external amount should be computed.
   * @param {Utxo[]} inputUtxos - The input UTXOs for the transaction.
   * @param {Utxo[]} outputUtxos - The output UTXOs for the transaction.
   * @param {string[]} assetPubkeysCircuit - An array of circuit public keys of assets.
   * @returns {BN} - The public amount of the asset.
   *
   */
  static getExternalAmount(
    assetIndex: number,
    // params: TransactionParameters,
    inputUtxos: Utxo[],
    outputUtxos: Utxo[],
    assetPubkeysCircuit: string[],
  ): BN {
    return new anchor.BN(0)
      .add(
        outputUtxos
          .filter((utxo: Utxo) => {
            return (
              utxo.assetsCircuit[assetIndex].toString() ==
              assetPubkeysCircuit![assetIndex]
            );
          })
          .reduce(
            (sum, utxo) =>
              // add all utxos of the same asset
              sum.add(utxo.amounts[assetIndex]),
            new anchor.BN(0),
          ),
      )
      .sub(
        inputUtxos
          .filter((utxo) => {
            return (
              utxo.assetsCircuit[assetIndex].toString() ==
              assetPubkeysCircuit[assetIndex]
            );
          })
          .reduce(
            (sum, utxo) => sum.add(utxo.amounts[assetIndex]),
            new anchor.BN(0),
          ),
      )
      .add(FIELD_SIZE)
      .mod(FIELD_SIZE);
  }

  /**
   * Computes the integrity Poseidon hash over transaction inputs that are not part of
   * the proof, but are included to prevent the relayer from changing any input of the
   * transaction.
   *
   * The hash is computed over the following inputs in the given order:
   * 1. Recipient SPL Account
   * 2. Recipient Solana Account
   * 3. Relayer Public Key
   * 4. Relayer Fee
   * 5. Encrypted UTXOs (limited to 512 bytes)
   *
   * @param {any} poseidon - Poseidon hash function instance.
   * @returns {Promise<BN>} A promise that resolves to the computed transaction integrity hash.
   * @throws {TransactionError} Throws an error if the relayer, recipient SPL or Solana accounts,
   * relayer fee, or encrypted UTXOs are undefined, or if the encryption of UTXOs fails.
   *
   * @example
   * ```typescript
   * const integrityHash = await getTxIntegrityHash(poseidonInstance);
   * ```
   */
  async getTxIntegrityHash(poseidon: any): Promise<BN> {
    if (!this.relayer)
      throw new TransactionError(
        TransactionErrorCode.RELAYER_UNDEFINED,
        "getTxIntegrityHash",
        "",
      );
    if (!this.accounts.recipientSpl)
      throw new TransactionError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getTxIntegrityHash",
        "",
      );
    if (!this.accounts.recipientSol)
      throw new TransactionError(
        TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
        "getTxIntegrityHash",
        "",
      );
    if (!this.relayer.getRelayerFee(this.ataCreationFee))
      throw new TransactionError(
        TransactionErrorCode.RELAYER_FEE_UNDEFINED,
        "getTxIntegrityHash",
        "",
      );

    if (!this.encryptedUtxos) {
      this.encryptedUtxos = await this.encryptOutUtxos(poseidon);
    }

    if (this.encryptedUtxos && this.encryptedUtxos.length > 512) {
      this.encryptedUtxos = this.encryptedUtxos.slice(0, 512);
    }
    if (this.encryptedUtxos) {
      const messageHash = this.message
        ? sha256(this.message)
        : new Uint8Array(32);
      // TODO(vadorovsky): Try to get rid of this hack during Verifier class
      // refactoring / removal
      // For example, we could derive which accounts exist in the IDL of the
      // verifier program method.
      const recipientSpl =
        this.verifierProgramId.toBase58() ===
        verifierProgramStorageProgramId.toBase58()
          ? new Uint8Array(32)
          : this.accounts.recipientSpl.toBytes();
      let hashInputBytes = new Uint8Array([
        ...messageHash,
        ...recipientSpl,
        ...this.accounts.recipientSol.toBytes(),
        ...this.relayer.accounts.relayerPubkey.toBytes(),
        ...this.relayer.getRelayerFee(this.ataCreationFee).toArray("le", 8),
        ...this.encryptedUtxos,
      ]);

      const hash = keccak_256
        .create({ dkLen: 32 })
        .update(Buffer.from(hashInputBytes))
        .digest();
      this.txIntegrityHash = new anchor.BN(hash).mod(FIELD_SIZE);

      return this.txIntegrityHash;
    } else {
      throw new TransactionError(
        TransactionErrorCode.ENCRYPTING_UTXOS_FAILED,
        "getTxIntegrityHash",
        "",
      );
    }
  }

  /**
   * @async
   *
   * This method is used to encrypt the output UTXOs.
   *
   * It first checks if there are encrypted UTXOs provided. If so, it uses those as the encrypted outputs.
   * If not, it goes through the output UTXOs for this transaction. If the UTXO has application data and this is to be included, it throws an error as this is currently not implemented.
   * Otherwise, it encrypts the UTXO and adds it to the list of encrypted outputs.
   *
   * Depending on the verifier configuration, it either combines two encrypted outputs into a single 256 byte output or adds padding to the encrypted outputs to ensure their length is correct.
   *
   * @param {any} poseidon - The poseidon hash function.
   * @param {Uint8Array} [encryptedUtxos] - An optional parameter for previously encrypted UTXOs.
   * @returns {Promise<Uint8Array>} - A Uint8Array of the encrypted output UTXOs.
   * @throws {TransactionError} - TransactionError: If automatic encryption for UTXOs with application data is attempted, as this is currently not implemented.
   *
   */
  async encryptOutUtxos(poseidon: any, encryptedUtxos?: Uint8Array) {
    let encryptedOutputs = new Array<any>();
    if (encryptedUtxos) {
      encryptedOutputs = Array.from(encryptedUtxos);
    } else if (this && this.outputUtxos) {
      for (var utxo in this.outputUtxos) {
        if (
          this.outputUtxos[utxo].appDataHash.toString() !== "0" &&
          this.outputUtxos[utxo].includeAppData
        )
          throw new TransactionError(
            TransactionErrorCode.UNIMPLEMENTED,
            "encryptUtxos",
            "Automatic encryption for utxos with application data is not implemented.",
          );
        encryptedOutputs.push(
          await this.outputUtxos[utxo].encrypt(
            poseidon,
            this.accounts.transactionMerkleTree,
            this.transactionNonce,
          ),
        );
      }
      if (
        this.verifierConfig.out == 2 &&
        encryptedOutputs[0].length + encryptedOutputs[1].length < 256
      ) {
        return new Uint8Array([
          ...encryptedOutputs[0],
          ...encryptedOutputs[1],
          ...new Array(
            256 - encryptedOutputs[0].length - encryptedOutputs[1].length,
          ).fill(0),
          // this is ok because these bytes are not sent and just added for the integrity hash
          // to be consistent, if the bytes were sent to the chain use rnd bytes for padding
        ]);
      } else {
        let tmpArray = new Array<any>();
        for (var i = 0; i < this.verifierConfig.out; i++) {
          tmpArray.push(...encryptedOutputs[i]);
          if (encryptedOutputs[i].length < 128) {
            // add random bytes for padding
            tmpArray.push(
              ...nacl.randomBytes(128 - encryptedOutputs[i].length),
            );
          }
        }

        if (tmpArray.length < 512) {
          tmpArray.push(
            ...nacl.randomBytes(
              this.verifierConfig.out * 128 - tmpArray.length,
            ),
          );
        }
        return new Uint8Array([...tmpArray]);
      }
    }
  }
}
