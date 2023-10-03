import { PublicKey, SystemProgram } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BN, BorshAccountsCoder, Program, Idl } from "@coral-xyz/anchor";
import {
  AUTHORITY,
  N_ASSET_PUBKEYS,
  STANDARD_SHIELDED_PRIVATE_KEY,
  STANDARD_SHIELDED_PUBLIC_KEY,
  verifierProgramStorageProgramId,
} from "../constants";
import { Utxo } from "../utxo";
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
  TransactionParametersError,
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
  createOutUtxos,
  BN_0,
} from "../index";
import { sha256 } from "@noble/hashes/sha256";
import { SPL_NOOP_PROGRAM_ID } from "@solana/spl-account-compression";
import nacl from "tweetnacl";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

type VerifierConfig = {
  in: number;
  out: number;
};

export class TransactionParameters implements transactionParameters {
  message?: Buffer;
  inputUtxos: Array<Utxo>;
  outputUtxos: Array<Utxo>;
  accounts: lightAccounts;
  relayer!: Relayer;
  encryptedUtxos?: Uint8Array;
  poseidon: any;
  publicAmountSpl: BN;
  publicAmountSol: BN;
  assetPubkeys: PublicKey[];
  assetPubkeysCircuit: string[];
  action: Action;
  ataCreationFee?: boolean;
  txIntegrityHash?: BN;
  verifierIdl: Idl;
  verifierProgramId: PublicKey;
  verifierConfig: VerifierConfig;
  account: Account;

  constructor({
    message,
    eventMerkleTreePubkey,
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
    ataCreationFee,
    verifierIdl,
    account,
  }: {
    message?: Buffer;
    eventMerkleTreePubkey: PublicKey;
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
    provider?: Provider;
    ataCreationFee?: boolean;
    verifierIdl: Idl;
    account: Account;
  }) {
    if (!outputUtxos && !inputUtxos) {
      throw new TransactionParametersError(
        TransactionErrorCode.NO_UTXOS_PROVIDED,
        "constructor",
        "",
      );
    }

    if (!verifierIdl) {
      throw new TransactionParametersError(
        TransactionParametersErrorCode.NO_VERIFIER_IDL_PROVIDED,
        "constructor",
        "",
      );
    }

    if (!poseidon) {
      throw new TransactionParametersError(
        TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
        "constructor",
        "",
      );
    }

    if (!action) {
      throw new TransactionParametersError(
        TransactionParametersErrorCode.NO_ACTION_PROVIDED,
        "constructor",
        "Define an action either Action.TRANSFER, Action.SHIELD,Action.UNSHIELD",
      );
    }
    this.account = account;
    this.verifierProgramId =
      TransactionParameters.getVerifierProgramId(verifierIdl);
    this.verifierConfig = TransactionParameters.getVerifierConfig(verifierIdl);
    this.message = message;
    this.verifierIdl = verifierIdl;
    this.poseidon = poseidon;
    this.ataCreationFee = ataCreationFee;
    this.encryptedUtxos = encryptedUtxos;
    this.action = action;
    this.inputUtxos = this.addEmptyUtxos(inputUtxos, this.verifierConfig.in);
    this.outputUtxos = this.addEmptyUtxos(outputUtxos, this.verifierConfig.out);
    if (action === Action.SHIELD && senderSol) {
      this.relayer = new Relayer(senderSol);
    } else if (action === Action.SHIELD && !senderSol) {
      throw new TransactionParametersError(
        TransactionErrorCode.SOL_SENDER_UNDEFINED,
        "constructor",
        "Sender sol always needs to be defined because we use it as the signer to instantiate the relayer object.",
      );
    }

    if (action !== Action.SHIELD) {
      if (relayer) {
        this.relayer = relayer;
      } else {
        throw new TransactionParametersError(
          TransactionErrorCode.RELAYER_UNDEFINED,
          "constructor",
          "For a transfer or unshield a relayer needs to be provided.",
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
    if (!this.publicAmountSol.gte(BN_0))
      throw new TransactionParametersError(
        TransactionParametersErrorCode.PUBLIC_AMOUNT_NEGATIVE,
        "constructor",
        "Public sol amount cannot be negative.",
      );
    if (!this.publicAmountSpl.gte(BN_0))
      throw new TransactionParametersError(
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
        throw new TransactionParametersError(
          TransactionParametersErrorCode.RELAYER_DEFINED,
          "constructor",
          "For a shield no relayer should to be provided, the user send the transaction herself.",
        );
      try {
        this.publicAmountSol.toArray("be", 8);
      } catch (error) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          "constructor",
          `Public amount sol ${this.publicAmountSol} needs to be a u64 at shield. Check whether you defined input and output utxos correctly, for a shield the amounts of output utxos need to be bigger than the amounts of input utxos`,
        );
      }

      try {
        this.publicAmountSpl.toArray("be", 8);
      } catch (error) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          "constructor",
          `Public amount spl ${this.publicAmountSpl} needs to be a u64 at shield. Check whether you defined input and output utxos correctly, for a shield the amounts of output utxos need to be bigger than the amounts of input utxos`,
        );
      }
      if (!this.publicAmountSol.eq(BN_0) && recipientSol) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          "constructor",
          "",
        );
      }
      if (!this.publicAmountSpl.eq(BN_0) && recipientSpl) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          "constructor",
          "",
        );
      }

      if (!this.publicAmountSol.eq(BN_0) && !senderSol) {
        throw new TransactionParametersError(
          TransactionErrorCode.SOL_SENDER_UNDEFINED,
          "constructor",
          "",
        );
      }
      if (!this.publicAmountSpl.eq(BN_0) && !senderSpl) {
        throw new TransactionParametersError(
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
        throw new TransactionParametersError(
          TransactionErrorCode.RELAYER_UNDEFINED,
          "constructor",
          "For an unshield, a relayer needs to be provided.",
        );
      // public amount is either 0 or negative
      // this.publicAmountSol.add(FIELD_SIZE).mod(FIELD_SIZE) this changes the value
      const tmpSol = this.publicAmountSol;
      if (!tmpSol.sub(FIELD_SIZE).lte(BN_0))
        throw new TransactionParametersError(
          TransactionParametersErrorCode.INVALID_PUBLIC_AMOUNT,
          "constructor",
          "",
        );
      const tmpSpl = this.publicAmountSpl;
      if (!tmpSpl.sub(FIELD_SIZE).lte(BN_0))
        throw new TransactionParametersError(
          TransactionParametersErrorCode.INVALID_PUBLIC_AMOUNT,
          "constructor",
          "",
        );
      try {
        if (!tmpSol.eq(BN_0)) {
          tmpSol.sub(FIELD_SIZE).toArray("be", 8);
        }
      } catch (error) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          "constructor",
          "Public amount needs to be a u64 at shield.",
        );
      }

      try {
        if (!tmpSpl.eq(BN_0)) {
          tmpSpl.sub(FIELD_SIZE).toArray("be", 8);
        }
      } catch (error) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_NOT_U64,
          "constructor",
          "Public amount needs to be a u64 at shield.",
        );
      }

      if (!this.publicAmountSol.eq(BN_0) && !recipientSol) {
        throw new TransactionParametersError(
          TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
          "constructor",
          "",
        );
      }

      if (!this.publicAmountSpl.eq(BN_0) && !recipientSpl) {
        throw new TransactionParametersError(
          TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
          "constructor",
          "",
        );
      }
      // && senderSol.toBase58() != merkle tree token pda
      if (!this.publicAmountSol.eq(BN_0) && senderSol) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          "constructor",
          "",
        );
      }
      if (!this.publicAmountSpl.eq(BN_0) && senderSpl) {
        throw new TransactionParametersError(
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
        throw new TransactionParametersError(
          TransactionErrorCode.RELAYER_UNDEFINED,
          "constructor",
          "For a transfer a relayer needs to be provided.",
        );
      if (!this.publicAmountSpl.eq(BN_0))
        throw new TransactionParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_SPL_NOT_ZERO,
          "constructor",
          `For a transfer public spl amount needs to be zero ${this.publicAmountSpl}`,
        );

      const tmpSol = this.publicAmountSol;

      if (
        !tmpSol
          .sub(FIELD_SIZE)
          .mul(new BN(-1))
          .eq(relayer.getRelayerFee(ataCreationFee))
      )
        throw new TransactionParametersError(
          TransactionParametersErrorCode.PUBLIC_AMOUNT_SOL_NOT_ZERO,
          "constructor",
          `public amount ${tmpSol
            .sub(FIELD_SIZE)
            .mul(new BN(-1))}  should be ${relayer.getRelayerFee(
            ataCreationFee,
          )}`,
        );

      if (recipientSpl) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          "constructor",
          "This is a transfer, no spl amount should be unshielded. To unshield an spl amount mark the transaction as unshield.",
        );
      }

      if (recipientSol) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          "constructor",
          "This is a transfer, no sol amount should be unshielded. To unshield an sol amount mark the transaction as unshield.",
        );
      }

      if (senderSol) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          "constructor",
          "",
        );
      }
      if (senderSpl) {
        throw new TransactionParametersError(
          TransactionParametersErrorCode.SPL_SENDER_DEFINED,
          "constructor",
          "",
        );
      }
    } else {
      throw new TransactionParametersError(
        TransactionParametersErrorCode.NO_ACTION_PROVIDED,
        "constructor",
        "",
      );
    }

    this.accounts = {
      systemProgramId: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      logWrapper: SPL_NOOP_PROGRAM_ID,
      eventMerkleTree: eventMerkleTreePubkey,
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
      senderSol: senderSol,
      recipientSol: recipientSol,
      programMerkleTree: merkleTreeProgramId,
      tokenAuthority: Transaction.getTokenAuthority(),
      verifierProgram: this.verifierProgramId,
    };

    this.assignAccounts();
    // @ts-ignore:
    this.accounts.signingAddress = this.relayer.accounts.relayerPubkey;
  }

  async toBytes(): Promise<Buffer> {
    let utxo;
    let coder = new BorshAccountsCoder(IDL_VERIFIER_PROGRAM_ZERO);
    let inputUtxosBytes: any[] = [];
    for (utxo of this.inputUtxos) {
      inputUtxosBytes.push(await utxo.toBytes());
    }
    let outputUtxosBytes: any[] = [];
    for (utxo of this.outputUtxos) {
      outputUtxosBytes.push(await utxo.toBytes());
    }
    let preparedObject = {
      outputUtxosBytes,
      inputUtxosBytes,
      relayerPubkey: this.relayer.accounts.relayerPubkey,
      relayerFee: this.relayer.relayerFee,
      ...this,
      ...this.accounts,
    };
    return await coder.encode("transactionParameters", preparedObject);
  }

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

  static getVerifierProgramId(verifierIdl: Idl): PublicKey {
    const programIdObj = verifierIdl.constants!.find(
      (constant) => constant.name === "PROGRAM_ID",
    );
    if (!programIdObj || typeof programIdObj.value !== "string") {
      throw new TransactionParametersError(
        TransactionParametersErrorCode.PROGRAM_ID_CONSTANT_UNDEFINED,
        'PROGRAM_ID constant not found in idl. Example: pub const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";',
      );
    }

    // Extracting the public key string value from the object and removing quotes.
    const programIdStr = programIdObj.value.slice(1, -1);
    return new PublicKey(programIdStr);
  }

  static getVerifierProgram(
    verifierIdl: Idl,
    anchorProvider: anchor.AnchorProvider,
  ): Program<Idl> {
    const programId = TransactionParameters.getVerifierProgramId(verifierIdl);
    const verifierProgram = new Program(verifierIdl, programId, anchorProvider);
    return verifierProgram;
  }

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
            throw new TransactionParametersError(
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
      throw new TransactionParametersError(
        TransactionParametersErrorCode.RELAYER_INVALID,
        "fromBytes",
        "The provided relayer has a different public key as the relayer publickey decoded from bytes",
      );
    }
    if (!relayer) {
      throw new TransactionParametersError(
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
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      verifierIdl: verifierIdl,
    });
  }

  static async getTxParams({
    tokenCtx,
    publicAmountSpl = BN_0,
    publicAmountSol = BN_0,
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
    appUtxo,
    addInUtxos = true,
    addOutUtxos = true,
    verifierIdl,
    mergeUtxos = false,
    message,
    assetLookupTable,
    verifierProgramLookupTable,
    separateSolUtxo = false,
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
    appUtxo?: AppUtxoConfig;
    addInUtxos?: boolean;
    addOutUtxos?: boolean;
    verifierIdl: Idl;
    mergeUtxos?: boolean;
    message?: Buffer;
    assetLookupTable: string[];
    verifierProgramLookupTable: string[];
    separateSolUtxo?: boolean;
  }): Promise<TransactionParameters> {
    if (action === Action.TRANSFER && !outUtxos && !mergeUtxos)
      throw new TransactionParametersError(
        UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED,
        "getTxParams",
        "Recipient outUtxo not provided for transfer",
      );

    if (action !== Action.SHIELD && !relayer?.getRelayerFee(ataCreationFee)) {
      // TODO: could make easier to read by adding separate if/cases
      throw new TransactionParametersError(
        RelayerErrorCode.RELAYER_FEE_UNDEFINED,
        "getTxParams",
        `No relayerFee provided for ${action.toLowerCase()}}`,
      );
    }
    if (!account) {
      throw new TransactionParametersError(
        CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
        "getTxParams",
        "account for change utxo is undefined",
      );
    }

    let inputUtxos: Utxo[] = inUtxos ? [...inUtxos] : [];
    let outputUtxos: Utxo[] = outUtxos ? [...outUtxos] : [];

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
        numberMaxOutUtxos:
          TransactionParameters.getVerifierConfig(verifierIdl).out,
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
        separateSolUtxo,
      });
    }

    let txParams = new TransactionParameters({
      outputUtxos,
      inputUtxos,
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      senderSpl: action === Action.SHIELD ? userSplAccount : undefined,
      senderSol:
        action === Action.SHIELD ? provider.wallet!.publicKey : undefined,
      recipientSpl: recipientSplAddress,
      recipientSol,
      poseidon: provider.poseidon,
      action,
      relayer: relayer,
      ataCreationFee,
      verifierIdl,
      message,
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      account,
    });

    return txParams;
  }

  /**
   * @description Adds empty utxos until the desired number of utxos is reached.
   * @note The zero knowledge proof circuit needs all inputs to be defined.
   * @note Therefore, we have to pass in empty inputs for values we don't use.
   * @param utxos
   * @param len
   * @returns
   */
  addEmptyUtxos(utxos: Utxo[] = [], len: number): Utxo[] {
    while (utxos.length < len) {
      utxos.push(
        new Utxo({
          poseidon: this.poseidon,
          publicKey: this.account.pubkey,
          assetLookupTable: [SystemProgram.programId.toBase58()],
          verifierProgramLookupTable: [SystemProgram.programId.toBase58()],
          isFillingUtxo: true,
        }),
      );
    }
    return utxos;
  }

  /**
   * @description Assigns spl and sol senderSpl or recipientSpl accounts to transaction parameters based on action.
   */
  assignAccounts() {
    if (!this.assetPubkeys)
      throw new TransactionParametersError(
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
        if (!this.publicAmountSpl?.eq(BN_0)) {
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
          !this.publicAmountSol.eq(BN_0) &&
          !this.publicAmountSol
            ?.sub(FIELD_SIZE)
            .mul(new BN(-1))
            .sub(new BN(this.relayer.getRelayerFee(this.ataCreationFee)))
            .eq(BN_0)
        ) {
          throw new TransactionParametersError(
            TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
            "assignAccounts",
            "Sol recipientSpl is undefined while public spl amount is != 0.",
          );
        }
      }
    } else if (this.action.toString() == Action.SHIELD.toString()) {
      this.accounts.recipientSpl = MerkleTreeConfig.getSplPoolPdaToken(
        this.assetPubkeys[1],
        merkleTreeProgramId,
      );
      this.accounts.recipientSol =
        MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;

      if (!this.accounts.senderSpl) {
        /// assigning a placeholder account
        this.accounts.senderSpl = AUTHORITY;
        if (!this.publicAmountSpl?.eq(BN_0)) {
          throw new TransactionParametersError(
            TransactionErrorCode.SPL_SENDER_UNDEFINED,
            "assignAccounts",
            "Spl senderSpl is undefined while public spl amount is != 0.",
          );
        }
      }
      this.accounts.senderSol = TransactionParameters.getEscrowPda(
        this.verifierProgramId,
      );
    } else {
      throw new TransactionParametersError(
        TransactionErrorCode.INVALID_ACTION,
        "assignAccounts",
        "Invalid action, supported actions are 'shield', 'unsield' and 'transfer'.",
      );
    }
  }

  static getEscrowPda(verifierProgramId: PublicKey): PublicKey {
    return PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("escrow")],
      verifierProgramId,
    )[0];
  }

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
        for (var _asset in assetPubkeysCircuit) {
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
      assetPubkeysCircuit.push(BN_0.toString());
      assetPubkeys.push(SystemProgram.programId);
    }

    return { assetPubkeysCircuit, assetPubkeys };
  }

  /**
   * @description Calculates the external amount for one asset.
   * @note This function might be too specific since the circuit allows assets to be in any index
   * @param assetIndex the index of the asset the external amount should be computed for
   * @returns {BN} the public amount of the asset
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
   * const integrityHash = await getTxIntegrityHash(poseidonInstance);
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
    if (
      this.encryptedUtxos &&
      this.encryptedUtxos.length > 128 * this.verifierConfig.out
    )
      throw new TransactionParametersError(
        TransactionParametersErrorCode.ENCRYPTED_UTXOS_TOO_LONG,
        "getTxIntegrityHash",
        `Encrypted utxos are too long: ${this.encryptedUtxos.length} > ${
          128 * this.verifierConfig.out
        }`,
      );

    if (!this.encryptedUtxos) {
      this.encryptedUtxos = await this.encryptOutUtxos(poseidon);
    }

    if (this.encryptedUtxos) {
      const relayerFee = new Uint8Array(
        this.relayer.getRelayerFee(this.ataCreationFee).toArray("le", 8),
      );

      let nullifiersHasher = sha256.create();
      this.inputUtxos.forEach((x) => {
        // const nullifier = x.getNullifier({ poseidon, account: this.account });
        // const nullifier = this.params.inputUtxos.map((x) => {

        // });
        let _account = this.account;
        if (x.publicKey.eq(STANDARD_SHIELDED_PUBLIC_KEY)) {
          _account = Account.fromPrivkey(
            poseidon,
            bs58.encode(STANDARD_SHIELDED_PRIVATE_KEY.toArray("be", 32)),
            bs58.encode(STANDARD_SHIELDED_PRIVATE_KEY.toArray("be", 32)),
            bs58.encode(STANDARD_SHIELDED_PRIVATE_KEY.toArray("be", 32)),
          );
        }
        const nullifier = x.getNullifier({
          poseidon: poseidon,
          account: _account,
        });
        if (nullifier) {
          let nullifierBytes = new anchor.BN(nullifier).toArray("be", 32);
          nullifiersHasher.update(new Uint8Array(nullifierBytes));
        }
      });
      const nullifiersHash = nullifiersHasher.digest();

      let leavesHasher = sha256.create();
      this.outputUtxos.forEach((x) => {
        const commitment = new anchor.BN(x.getCommitment(poseidon)).toArray(
          "be",
          32,
        );
        leavesHasher.update(new Uint8Array(commitment));
      });
      const leavesHash = leavesHasher.digest();

      const messageHash = this.message
        ? sha256(this.message)
        : new Uint8Array(32);
      const encryptedUtxosHash = sha256
        .create()
        .update(this.encryptedUtxos)
        .digest();

      const amountHash = sha256
        .create()
        .update(new Uint8Array(this.publicAmountSol.toArray("be", 32)))
        .update(new Uint8Array(this.publicAmountSpl.toArray("be", 32)))
        .update(relayerFee)
        .digest();

      const eventHash = sha256
        .create()
        .update(nullifiersHash)
        .update(leavesHash)
        .update(messageHash)
        .update(encryptedUtxosHash)
        .update(amountHash)
        .digest();

      // TODO(vadorovsky): Try to get rid of this hack during Verifier class
      // refactoring / removal
      // For example, we could derive which accounts exist in the IDL of the
      // verifier program method.
      const recipientSpl =
        this.verifierProgramId.toBase58() ===
        verifierProgramStorageProgramId.toBase58()
          ? new Uint8Array(32)
          : this.accounts.recipientSpl.toBytes();

      const hash = sha256
        .create()
        .update(eventHash)
        .update(recipientSpl)
        .update(this.accounts.recipientSol.toBytes())
        .update(this.relayer.accounts.relayerPubkey.toBytes())
        .update(relayerFee)
        .update(this.encryptedUtxos)
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

  async encryptOutUtxos(poseidon: any, encryptedUtxos?: Uint8Array) {
    let encryptedOutputs = new Array<any>();
    if (encryptedUtxos) {
      encryptedOutputs = Array.from(encryptedUtxos);
    } else if (this && this.outputUtxos) {
      for (const utxo in this.outputUtxos) {
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
          await this.outputUtxos[utxo].encrypt({
            poseidon,
            account: this.account,
            merkleTreePdaPublicKey: this.accounts.transactionMerkleTree,
          }),
        );
      }
      encryptedOutputs = encryptedOutputs
        .map((elem) => Array.from(elem))
        .flat();
      if (
        encryptedOutputs.length < 128 * this.verifierConfig.out &&
        this.verifierConfig.out === 2
      ) {
        return new Uint8Array([
          ...encryptedOutputs,
          ...new Array(
            128 * this.verifierConfig.out - encryptedOutputs.length,
          ).fill(0),
          // for verifier zero and one these bytes are not sent and just added for the integrity hash
          // to be consistent, if the bytes were sent to the chain use rnd bytes for padding
        ]);
      }
      if (encryptedOutputs.length < 128 * this.verifierConfig.out) {
        return new Uint8Array([
          ...encryptedOutputs,
          ...nacl.randomBytes(
            128 * this.verifierConfig.out - encryptedOutputs.length,
          ),
        ]);
      }
    }
  }

  getTransactionHash(poseidon: any): string {
    if (!this.txIntegrityHash)
      throw new TransactionError(
        TransactionErrorCode.TX_INTEGRITY_HASH_UNDEFINED,
        "getTransactionHash",
      );
    const inputHasher = poseidon.F.toString(
      poseidon(this?.inputUtxos?.map((utxo) => utxo.getCommitment(poseidon))),
    );
    const outputHasher = poseidon.F.toString(
      poseidon(this?.outputUtxos?.map((utxo) => utxo.getCommitment(poseidon))),
    );
    const transactionHash = poseidon.F.toString(
      poseidon([inputHasher, outputHasher, this.txIntegrityHash.toString()]),
    );
    return transactionHash;
  }
}
