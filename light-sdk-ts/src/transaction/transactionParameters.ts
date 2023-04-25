import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
import {
  AUTHORITY,
  MERKLE_TREE_KEY,
  verifierProgramZeroProgramId,
} from "../constants";
import { N_ASSET_PUBKEYS, Utxo } from "../utxo";
import { Verifier, VerifierZero } from "../verifiers";
import { MerkleTreeConfig } from "../merkleTree/merkleTreeConfig";
import {
  FIELD_SIZE,
  hashAndTruncateToCircuit,
  Account,
  merkleTreeProgramId,
  Relayer,
  TransactionErrorCode,
  TransactionError,
  TransactioParametersError,
  TransactionParametersErrorCode,
  Provider,
  Recipient,
  UserError,
  UserErrorCode,
  RelayerErrorCode,
  CreateUtxoErrorCode,
  selectInUtxos,
  createMissingOutUtxos,
  Transaction,
  Action,
  TokenContext,
  transactionParameters,
  lightAccounts,
  IDL_VERIFIER_PROGRAM_ZERO,
  AppUtxoConfig,
  validateUtxoAmounts,
} from "../index";

export class TransactionParameters implements transactionParameters {
  inputUtxos: Array<Utxo>;
  outputUtxos: Array<Utxo>;
  accounts: lightAccounts;
  // @ts-ignore:
  relayer: Relayer;
  encryptedUtxos?: Uint8Array;
  verifier: Verifier;
  poseidon: any;
  publicAmountSpl: BN;
  publicAmountSol: BN;
  assetPubkeys: PublicKey[];
  assetPubkeysCircuit: string[];
  action: Action;
  ataCreationFee?: boolean;
  transactionIndex: number;

  constructor({
    merkleTreePubkey,
    verifier,
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
    transactionIndex,
    validateUtxos = true,
  }: {
    merkleTreePubkey: PublicKey;
    verifier: Verifier;
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
    transactionIndex: number;
    validateUtxos?: boolean;
  }) {
    if (!outputUtxos && !inputUtxos) {
      throw new TransactioParametersError(
        TransactionErrorCode.NO_UTXOS_PROVIDED,
        "constructor",
        "",
      );
    }

    if (!verifier) {
      throw new TransactioParametersError(
        TransactionParametersErrorCode.NO_VERIFIER_PROVIDED,
        "constructor",
        "",
      );
    }
    if (!verifier.verifierProgram)
      throw new TransactioParametersError(
        TransactionErrorCode.VERIFIER_PROGRAM_UNDEFINED,
        "constructor",
        "verifier.program undefined",
      );

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

    this.transactionIndex = transactionIndex;
    this.verifier = verifier;
    this.poseidon = poseidon;
    this.ataCreationFee = ataCreationFee;
    this.encryptedUtxos = encryptedUtxos;
    this.action = action;
    this.inputUtxos = this.addEmptyUtxos(inputUtxos, this.verifier.config.in);
    this.outputUtxos = this.addEmptyUtxos(
      outputUtxos,
      this.verifier.config.out,
    );

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
      transactionMerkleTree: merkleTreePubkey,
      registeredVerifierPda: Transaction.getRegisteredVerifierPda(
        merkleTreeProgramId,
        verifier.verifierProgram.programId,
      ),
      authority: Transaction.getSignerAuthorityPda(
        merkleTreeProgramId,
        verifier.verifierProgram.programId,
      ),
      senderSpl: senderSpl,
      recipientSpl: recipientSpl,
      senderSol: senderSol, // TODO: change to senderSol
      recipientSol: recipientSol, // TODO: change name to recipientSol
      programMerkleTree: merkleTreeProgramId,
      tokenAuthority: Transaction.getTokenAuthority(),
    };

    this.assignAccounts();
    // @ts-ignore:
    this.accounts.signingAddress = this.relayer.accounts.relayerPubkey;
  }

  async toBytes(): Promise<Buffer> {
    let coder = new BorshAccountsCoder(IDL_VERIFIER_PROGRAM_ZERO);
    let inputUtxosBytes = [];
    for (var utxo of this.inputUtxos) {
      inputUtxosBytes.push(await utxo.toBytes());
    }
    let outputUtxosBytes = [];
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
      transactionIndex: new BN(this.transactionIndex),
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
          constant.name === "programId" &&
          constant.type === "string" &&
          constant.value === `"${programId}"`
        ) {
          return i;
        }
      }
    }

    return -1; // Return -1 if the programId is not found in any IDL object
  }

  static async fromBytes({
    poseidon,
    utxoIdls,
    bytes,
    verifier,
    relayer,
  }: {
    poseidon: any;
    utxoIdls?: anchor.Idl[];
    bytes: Buffer;
    verifier: Verifier;
    relayer: Relayer;
  }): Promise<TransactionParameters> {
    let coder = new BorshAccountsCoder(IDL_VERIFIER_PROGRAM_ZERO);
    let decoded = coder.decodeUnchecked("transactionParameters", bytes);

    const getUtxos = (
      utxoBytesArray: Array<Buffer>,
      utxoIdls?: anchor.Idl[],
    ) => {
      let utxos: Utxo[] = [];
      for (var [_, utxoBytes] of utxoBytesArray.entries()) {
        let appDataIdl = undefined;
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
      verifier,
      relayer,
      ...decoded,
      action,
      merkleTreePubkey: MERKLE_TREE_KEY,
    });
  }

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
    transactionIndex,
    appUtxo,
    addInUtxos = true,
    addOutUtxos = true,
    verifier,
  }: {
    tokenCtx: TokenContext;
    publicAmountSpl?: BN;
    publicAmountSol?: BN;
    userSplAccount?: PublicKey;
    account?: Account;
    utxos?: Utxo[];
    recipientSol?: PublicKey;
    recipientSplAddress?: PublicKey;
    inUtxos?: Utxo[];
    outUtxos?: Utxo[];
    action: Action;
    provider: Provider;
    relayer?: Relayer;
    ataCreationFee?: boolean;
    transactionIndex: number;
    appUtxo?: AppUtxoConfig;
    addInUtxos?: boolean;
    addOutUtxos?: boolean;
    verifier: Verifier;
  }): Promise<TransactionParameters> {
    publicAmountSol = publicAmountSol ? publicAmountSol : new BN(0);
    publicAmountSpl = publicAmountSpl ? publicAmountSpl : new BN(0);

    if (action === Action.TRANSFER && !outUtxos)
      throw new UserError(
        UserErrorCode.SHIELDED_RECIPIENT_UNDEFINED,
        "getTxParams",
        "Recipient outUtxo not provided for transfer",
      );

    if (action !== Action.SHIELD && !relayer?.getRelayerFee(ataCreationFee)) {
      // TODO: could make easier to read by adding separate if/cases
      throw new UserError(
        RelayerErrorCode.RELAYER_FEE_UNDEFINED,
        "getTxParams",
        `No relayerFee provided for ${action.toLowerCase()}}`,
      );
    }
    if (!account) {
      throw new UserError(
        CreateUtxoErrorCode.ACCOUNT_UNDEFINED,
        "getTxParams",
        "Account not defined",
      );
    }

    var inputUtxos: Utxo[] = inUtxos ? [...inUtxos] : [];
    var outputUtxos: Utxo[] = outUtxos ? [...outUtxos] : [];

    if (addInUtxos) {
      inputUtxos = selectInUtxos({
        publicMint: tokenCtx.tokenAccount,
        publicAmountSpl,
        publicAmountSol,
        poseidon: provider.poseidon,
        inUtxos,
        outUtxos,
        utxos,
        relayerFee: relayer?.getRelayerFee(ataCreationFee),
        action,
        numberMaxInUtxos: verifier.config.in,
      });
    }
    if (addOutUtxos) {
      outputUtxos = createOutUtxos({
        publicMint: tokenCtx.tokenAccount,
        publicAmountSpl,
        inUtxos: inputUtxos,
        publicAmountSol, // TODO: add support for extra sol for unshield & transfer
        poseidon: provider.poseidon,
        relayerFee: relayer?.getRelayerFee(ataCreationFee),
        changeUtxoAccount: account,
        outUtxos,
        action,
        appUtxo,
        numberMaxOutUtxos: verifier.config.out,
      });
    }

    let txParams = new TransactionParameters({
      outputUtxos,
      inputUtxos,
      merkleTreePubkey: MERKLE_TREE_KEY,
      senderSpl: action === Action.SHIELD ? userSplAccount : undefined,
      senderSol:
        action === Action.SHIELD ? provider.wallet!.publicKey : undefined,
      recipientSpl: recipientSplAddress,
      recipientSol,
      verifier,
      poseidon: provider.poseidon,
      action,
      lookUpTable: provider.lookUpTable!,
      relayer: relayer,
      ataCreationFee,
      transactionIndex,
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
      utxos.push(new Utxo({ poseidon: this.poseidon }));
    }
    return utxos;
  }

  /**
   * @description Assigns spl and sol senderSpl or recipientSpl accounts to transaction parameters based on action.
   */
  assignAccounts() {
    if (!this.verifier.verifierProgram)
      throw new TransactioParametersError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "assignAccounts",
        "Verifier.verifierProgram undefined.",
      );
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
        // assigning a placeholder account
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
        this.verifier.verifierProgram.programId,
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
}
