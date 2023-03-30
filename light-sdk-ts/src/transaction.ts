import {
  PublicKey,
  SystemProgram,
  ComputeBudgetProgram,
  AddressLookupTableAccount,
  TransactionMessage,
  VersionedTransaction,
  TransactionSignature,
  TransactionInstruction,
  Transaction as SolanaTransaction,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { BN, Program } from "@coral-xyz/anchor";
import { AUTHORITY, confirmConfig, MERKLE_TREE_KEY } from "./constants";
import { N_ASSET_PUBKEYS, Utxo } from "./utxo";
import { PublicInputs, Verifier } from "./verifiers";
import { checkRentExemption } from "./test-utils/testChecks";
import { MerkleTreeConfig } from "./merkleTree/merkleTreeConfig";
import {
  FIELD_SIZE,
  hashAndTruncateToCircuit,
  Account,
  merkleTreeProgramId,
  Relayer,
  TransactionErrorCode,
  TransactionError,
  ProviderErrorCode,
  SolMerkleTreeErrorCode,
  TransactioParametersError,
  initLookUpTable,
  TransactionParametersErrorCode,
  Provider,
  ADMIN_AUTH_KEYPAIR,
  sendVersionedTransaction,
} from "./index";
import { IDL_MERKLE_TREE_PROGRAM } from "./idls/index";
const snarkjs = require("snarkjs");
const nacl = require("tweetnacl");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, stringifyBigInts, leInt2Buff, leBuff2int } =
  ffjavascript.utils;
const { keccak_256 } = require("@noble/hashes/sha3");
var assert = require("assert");

export const createEncryptionKeypair = () => nacl.box.keyPair();

export type transactionParameters = {
  provider?: Provider;
  inputUtxos?: Array<Utxo>;
  outputUtxos?: Array<Utxo>;
  accounts: {
    sender?: PublicKey;
    recipient?: PublicKey;
    senderFee?: PublicKey;
    recipientFee?: PublicKey;
    verifierState?: PublicKey;
    tokenAuthority?: PublicKey;
  };
  relayer?: Relayer;
  encryptedUtxos?: Uint8Array;
  verifier: Verifier;
  nullifierPdaPubkeys?: {
    isSigner: boolean;
    isWritable: boolean;
    pubkey: PublicKey;
  }[];
  leavesPdaPubkeys?: {
    isSigner: boolean;
    isWritable: boolean;
    pubkey: PublicKey;
  }[];
};

export enum Action {
  SHIELD = "SHIELD",
  TRANSFER = "TRANSFER",
  UNSHIELD = "UNSHIELD",
}

export type lightAccounts = {
  sender?: PublicKey;
  recipient?: PublicKey;
  senderFee?: PublicKey;
  recipientFee?: PublicKey;
  verifierState?: PublicKey;
  tokenAuthority?: PublicKey;
  systemProgramId: PublicKey;
  merkleTree: PublicKey;
  tokenProgram: PublicKey;
  registeredVerifierPda: PublicKey;
  authority: PublicKey;
  signingAddress?: PublicKey;
  programMerkleTree: PublicKey;
};

export type remainingAccount = {
  isSigner: boolean;
  isWritable: boolean;
  pubkey: PublicKey;
};

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

  constructor({
    merkleTreePubkey,
    verifier,
    sender,
    recipient,
    senderFee,
    recipientFee,
    inputUtxos,
    outputUtxos,
    relayer,
    encryptedUtxos,
    poseidon,
    action,
    lookUpTable,
    ataCreationFee,
  }: {
    merkleTreePubkey: PublicKey;
    verifier: Verifier;
    sender?: PublicKey;
    recipient?: PublicKey;
    senderFee?: PublicKey;
    recipientFee?: PublicKey;
    inputUtxos?: Utxo[];
    outputUtxos?: Utxo[];
    relayer?: Relayer;
    encryptedUtxos?: Uint8Array;
    poseidon: any;
    action: Action;
    lookUpTable?: PublicKey;
    provider?: Provider;
    ataCreationFee?: boolean;
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

    if (action === Action.SHIELD && senderFee && lookUpTable) {
      this.relayer = new Relayer(senderFee, lookUpTable);
    } else if (action === Action.SHIELD && !senderFee) {
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
       * sender is the user
       * recipient is the merkle tree
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
      if (!this.publicAmountSol.eq(new BN(0)) && recipientFee) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          "constructor",
          "",
        );
      }
      if (!this.publicAmountSpl.eq(new BN(0)) && recipient) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          "constructor",
          "",
        );
      }

      if (!this.publicAmountSol.eq(new BN(0)) && !senderFee) {
        throw new TransactioParametersError(
          TransactionErrorCode.SOL_SENDER_UNDEFINED,
          "constructor",
          "",
        );
      }
      if (!this.publicAmountSpl.eq(new BN(0)) && !sender) {
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
       * for public amounts greater than 0 a recipient needs to be defined
       * sender is the merkle tree
       * recipient is the user
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

      if (!this.publicAmountSol.eq(new BN(0)) && !recipientFee) {
        throw new TransactioParametersError(
          TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
          "constructor",
          "",
        );
      }

      if (!this.publicAmountSpl.eq(new BN(0)) && !recipient) {
        throw new TransactioParametersError(
          TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
          "constructor",
          "",
        );
      }
      // && senderFee.toBase58() != merkle tree token pda
      if (!this.publicAmountSol.eq(new BN(0)) && senderFee) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          "constructor",
          "",
        );
      }
      if (!this.publicAmountSpl.eq(new BN(0)) && sender) {
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
       * sender is the merkle tree
       * recipient does not exists it is an internal transfer just the relayer is paid
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

      if (recipient) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SPL_RECIPIENT_DEFINED,
          "constructor",
          "This is a transfer, no spl amount should be withdrawn. To withdraw an spl amount mark the transaction as withdrawal.",
        );
      }

      if (recipientFee) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SOL_RECIPIENT_DEFINED,
          "constructor",
          "This is a transfer, no sol amount should be withdrawn. To withdraw an sol amount mark the transaction as withdrawal.",
        );
      }

      if (senderFee) {
        throw new TransactioParametersError(
          TransactionParametersErrorCode.SOL_SENDER_DEFINED,
          "constructor",
          "",
        );
      }
      if (sender) {
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
      merkleTree: merkleTreePubkey,
      registeredVerifierPda: Transaction.getRegisteredVerifierPda(
        merkleTreeProgramId,
        verifier.verifierProgram.programId,
      ),
      authority: Transaction.getSignerAuthorityPda(
        merkleTreeProgramId,
        verifier.verifierProgram.programId,
      ),
      sender: sender,
      recipient: recipient,
      senderFee: senderFee, // TODO: change to senderSol
      recipientFee: recipientFee, // TODO: change name to recipientSol
      programMerkleTree: merkleTreeProgramId,
      tokenAuthority: Transaction.getTokenAuthority(),
    };

    this.assignAccounts();
    // @ts-ignore:
    this.accounts.signingAddress = this.relayer.accounts.relayerPubkey;
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
   * @description Assigns spl and sol sender or recipient accounts to transaction parameters based on action.
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
      this.accounts.sender = MerkleTreeConfig.getSplPoolPdaToken(
        this.assetPubkeys[1],
        merkleTreeProgramId,
      );
      this.accounts.senderFee =
        MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;

      if (!this.accounts.recipient) {
        // AUTHORITY is used as place holder
        this.accounts.recipient = AUTHORITY;
        if (!this.publicAmountSpl?.eq(new BN(0))) {
          throw new TransactionError(
            TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
            "assignAccounts",
            "Spl recipient is undefined while public spl amount is != 0.",
          );
        }
      }

      if (!this.accounts.recipientFee) {
        // AUTHORITY is used as place holder
        this.accounts.recipientFee = AUTHORITY;
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
            "Sol recipient is undefined while public spl amount is != 0.",
          );
        }
      }
    } else {
      if (this.action.toString() !== Action.SHIELD.toString()) {
        throw new TransactioParametersError(
          TransactionErrorCode.ACTION_IS_NO_DEPOSIT,
          "assignAccounts",
          "Action is withdrawal but should not be. Spl & sol sender accounts are provided and a relayer which is used to identify transfers and withdrawals. For a deposit do not provide a relayer.",
        );
      }

      this.accounts.recipient = MerkleTreeConfig.getSplPoolPdaToken(
        this.assetPubkeys[1],
        merkleTreeProgramId,
      );
      this.accounts.recipientFee =
        MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda;
      if (!this.accounts.sender) {
        // assigning a placeholder account
        this.accounts.sender = AUTHORITY;
        if (!this.publicAmountSpl?.eq(new BN(0))) {
          throw new TransactioParametersError(
            TransactionErrorCode.SPL_SENDER_UNDEFINED,
            "assignAccounts",
            "Spl sender is undefined while public spl amount is != 0.",
          );
        }
      }
      this.accounts.senderFee = TransactionParameters.getEscrowPda(
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
  // TODO: write test
  // TODO: rename to publicAmount
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
// TODO: make dev provide the classification and check here -> it is easier to check whether transaction parameters are plausible

// add verifier class which is passed in with the constructor
// this class replaces the send transaction, also configures path the provingkey and witness, the inputs for the integrity hash
// input custom verifier with three functions by default prepare, proof, send
// include functions from sdk in shieldedTransaction

// TODO: add log option that enables logs
// TODO: write functional test for every method
export class Transaction {
  merkleTreeProgram?: Program<typeof IDL_MERKLE_TREE_PROGRAM>;
  shuffleEnabled: Boolean;
  params: TransactionParameters; // contains accounts
  appParams?: any;
  // TODO: relayer shd pls should be part of the provider by default + optional override on Transaction level
  provider: Provider;

  transactionInputs: {
    publicInputs?: PublicInputs;
    rootIndex?: any;
    proofBytes?: any;
    proofBytesApp?: any;
    publicInputsApp?: PublicInputs;
    encryptedUtxos?: Uint8Array;
  };

  remainingAccounts?: {
    nullifierPdaPubkeys?: remainingAccount[];
    leavesPdaPubkeys?: remainingAccount[];
  };

  proofInput: any;
  proofInputSystem: any;

  // Tests
  testValues?: {
    recipientBalancePriorTx?: BN;
    relayerRecipientAccountBalancePriorLastTx?: BN;
    txIntegrityHash?: BN;
    senderFeeBalancePriorTx?: BN;
    recipientFeeBalancePriorTx?: BN;
    is_token?: boolean;
  };

  /**
   * Initialize transaction
   *
   * @param relayer recipient of the unshielding
   * @param shuffleEnabled
   */
  constructor({
    provider,
    shuffleEnabled = false,
    params,
    appParams,
  }: {
    provider: Provider;
    shuffleEnabled?: boolean;
    params: TransactionParameters;
    appParams?: any;
  }) {
    if (!provider)
      throw new TransactionError(
        TransactionErrorCode.PROVIDER_UNDEFINED,
        "constructor",
      );
    if (!provider.poseidon)
      throw new TransactionError(
        TransactionErrorCode.POSEIDON_HASHER_UNDEFINED,
        "constructor",
        "Poseidon hasher in provider undefined.",
      );
    if (!provider.solMerkleTree)
      throw new TransactionError(
        ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
        "constructor",
        "Merkle tree not set in provider",
      );
    if (!provider.wallet)
      throw new TransactionError(
        TransactionErrorCode.WALLET_UNDEFINED,
        "constructor",
        "Wallet not set in provider.",
      );
    if (!params) {
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "constructor",
      );
    }
    this.provider = provider;

    this.shuffleEnabled = shuffleEnabled;
    // TODO: create and check for existence of merkleTreeAssetPubkey depending on utxo asset
    this.params = params;
    this.appParams = appParams;

    //TODO: change to check whether browser/node wallet are the same as signing address
    if (params.action === Action.SHIELD) {
      let wallet = this.provider.wallet;
      if (
        wallet?.publicKey.toBase58() !==
          params.relayer.accounts.relayerPubkey.toBase58() &&
        wallet?.publicKey.toBase58() !==
          params.accounts.signingAddress?.toBase58()
      ) {
        throw new TransactionError(
          TransactionErrorCode.WALLET_RELAYER_INCONSISTENT,
          "constructor",
          `Node or Browser wallet and senderFee used to instantiate yourself as relayer at deposit are inconsistent.`,
        );
      }
    }

    this.transactionInputs = {};
    this.testValues = {};
    this.remainingAccounts = {};
  }

  /** Returns serialized instructions */
  async proveAndCreateInstructionsJson(): Promise<string[]> {
    await this.compileAndProve();
    return await this.getInstructionsJson();
  }

  async proveAndCreateInstructions(): Promise<TransactionInstruction[]> {
    await this.compileAndProve();
    if (this.appParams) {
      return await this.appParams.verifier.getInstructions(this);
    } else if (this.params) {
      return await this.params.verifier.getInstructions(this);
    } else {
      throw new TransactionError(
        TransactionErrorCode.NO_PARAMETERS_PROVIDED,
        "proveAndCreateInstructions",
        "",
      );
    }
  }

  async compileAndProve() {
    this.compile();
    await this.getProof();
    if (this.appParams) {
      await this.getAppProof();
    }
    await this.getRootIndex();
    await this.getPdaAddresses();
  }

  /**
   * @description Prepares proof inputs.
   */
  compile() {
    this.shuffleUtxos(this.params.inputUtxos);
    this.shuffleUtxos(this.params.outputUtxos);

    if (!this.provider.solMerkleTree)
      throw new TransactionError(
        ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
        "getProofInput",
        "",
      );

    const { inputMerklePathIndices, inputMerklePathElements } =
      Transaction.getMerkleProofs(this.provider, this.params.inputUtxos);

    this.proofInputSystem = {
      root: this.provider.solMerkleTree.merkleTree.root(),
      inputNullifier: this.params.inputUtxos.map((x) => x.getNullifier()),
      // TODO: move public and fee amounts into tx preparation
      publicAmount: this.params.publicAmountSpl.toString(),
      feeAmount: this.params.publicAmountSol.toString(),
      mintPubkey: this.getMint(),
      inPrivateKey: this.params.inputUtxos?.map((x) => x.account.privkey),
      inPathIndices: inputMerklePathIndices,
      inPathElements: inputMerklePathElements,
    };
    this.proofInput = {
      extDataHash: this.getTxIntegrityHash().toString(),
      outputCommitment: this.params.outputUtxos.map((x) => x.getCommitment()),
      inAmount: this.params.inputUtxos?.map((x) => x.amounts),
      inBlinding: this.params.inputUtxos?.map((x) => x.blinding),
      assetPubkeys: this.params.assetPubkeysCircuit,
      outAmount: this.params.outputUtxos?.map((x) => x.amounts),
      outBlinding: this.params.outputUtxos?.map((x) => x.blinding),
      outPubkey: this.params.outputUtxos?.map((x) => x.account.pubkey),
      inIndices: this.getIndices(this.params.inputUtxos),
      outIndices: this.getIndices(this.params.outputUtxos),
      inInstructionType: this.params.inputUtxos?.map((x) => x.instructionType),
      outInstructionType: this.params.outputUtxos?.map(
        (x) => x.instructionType,
      ),
      inPoolType: this.params.inputUtxos?.map((x) => x.poolType),
      outPoolType: this.params.outputUtxos?.map((x) => x.poolType),
      inVerifierPubkey: this.params.inputUtxos?.map(
        (x) => x.verifierAddressCircuit,
      ),
      outVerifierPubkey: this.params.outputUtxos?.map(
        (x) => x.verifierAddressCircuit,
      ),
    };
    if (this.appParams) {
      this.proofInput.connectingHash = Transaction.getConnectingHash(
        this.params,
        this.provider.poseidon,
        this.proofInput.extDataHash,
      );
      this.proofInput.verifier = this.params.verifier?.pubkey;
    }
  }

  getMint() {
    if (this.params.publicAmountSpl.toString() == "0") {
      return new BN(0);
    } else if (this.params.assetPubkeysCircuit) {
      return this.params.assetPubkeysCircuit[1];
    } else {
      throw new TransactionError(
        TransactionErrorCode.GET_MINT_FAILED,
        "getMint",
        "Get mint failed, transaction parameters dont contain assetPubkeysCircuit but should after normal instantiation",
      );
    }
  }

  async getAppProof() {
    if (!this.appParams)
      throw new TransactionError(
        TransactionErrorCode.APP_PARAMETERS_UNDEFINED,
        "getAppProof",
        "",
      );

    this.appParams.inputs.connectingHash = Transaction.getConnectingHash(
      this.params,
      this.provider.poseidon,
      this.getTxIntegrityHash().toString(),
    );
    const path = require("path");
    // TODO: find a better more flexible solution, pass in path with app params
    const firstPath = path.resolve(__dirname, "../../../sdk/build-circuit/");
    let { proofBytes, publicInputs } = await this.getProofInternal(
      this.appParams.verifier,
      {
        ...this.appParams.inputs,
        ...this.proofInput,
        inPublicKey: this.params?.inputUtxos?.map(
          (utxo) => utxo.account.pubkey,
        ),
      },
      firstPath,
    );

    this.transactionInputs.proofBytesApp = proofBytes;
    this.transactionInputs.publicInputsApp = publicInputs;
  }

  async getProof() {
    const path = require("path");
    const firstPath = path.resolve(__dirname, "../build-circuits/");

    if (!this.params.verifier)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_UNDEFINED,
        "getProof",
        "",
      );

    let { proofBytes, publicInputs } = await this.getProofInternal(
      this.params?.verifier,
      { ...this.proofInput, ...this.proofInputSystem },
      firstPath,
    );
    this.transactionInputs.proofBytes = proofBytes;
    this.transactionInputs.publicInputs = publicInputs;
  }

  async getProofInternal(verifier: Verifier, inputs: any, firstPath: string) {
    if (!this.proofInput)
      throw new TransactionError(
        TransactionErrorCode.PROOF_INPUT_UNDEFINED,
        "transaction not compiled",
      );

    const completePathWtns = firstPath + "/" + verifier.wtnsGenPath;
    const completePathZkey = firstPath + "/" + verifier.zkeyPath;
    console.time("Proof generation");

    const { proof, publicSignals } = await snarkjs.groth16.fullProve(
      stringifyBigInts(inputs),
      completePathWtns,
      completePathZkey,
    );
    console.timeEnd("Proof generation");

    const vKey = await snarkjs.zKey.exportVerificationKey(completePathZkey);
    const res = await snarkjs.groth16.verify(vKey, publicSignals, proof);
    if (res !== true) {
      throw new TransactionError(
        TransactionErrorCode.INVALID_PROOF,
        "getProofInternal",
      );
    }
    const proofJson = JSON.stringify(proof, null, 1);
    const publicInputsJson = JSON.stringify(publicSignals, null, 1);
    try {
      var publicInputsBytesJson = JSON.parse(publicInputsJson.toString());
      var publicInputsBytes = new Array<Array<number>>();
      for (var i in publicInputsBytesJson) {
        let ref: Array<number> = Array.from([
          ...leInt2Buff(unstringifyBigInts(publicInputsBytesJson[i]), 32),
        ]).reverse();
        publicInputsBytes.push(ref);
        // TODO: replace ref, error is that le and be do not seem to be consistent
        // new BN(publicInputsBytesJson[i], "le").toArray("be",32)
        // assert.equal(ref.toString(), publicInputsBytes[publicInputsBytes.length -1].toString());
      }
      const publicInputs =
        verifier.parsePublicInputsFromArray(publicInputsBytes);

      const proofBytes = await Transaction.parseProofToBytesArray(proofJson);
      return { proofBytes, publicInputs };
    } catch (error) {
      console.error("error while generating and validating proof");
      throw error;
    }
  }

  static getConnectingHash(
    params: TransactionParameters,
    poseidon: any,
    txIntegrityHash: any,
  ): string {
    const inputHasher = poseidon.F.toString(
      poseidon(params?.inputUtxos?.map((utxo) => utxo.getCommitment())),
    );
    const outputHasher = poseidon.F.toString(
      poseidon(params?.outputUtxos?.map((utxo) => utxo.getCommitment())),
    );
    const connectingHash = poseidon.F.toString(
      poseidon([inputHasher, outputHasher, txIntegrityHash.toString()]),
    );
    return connectingHash;
  }

  // TODO: add index to merkle tree and check correctness at setup
  // TODO: repeat check for example at tx init to acertain that the merkle tree is not outdated
  /**
   * @description fetches the merkle tree pda from the chain and checks in which index the root of the local merkle tree is.
   */
  async getRootIndex() {
    if (!this.provider.solMerkleTree)
      throw new TransactionError(
        ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
        "getRootIndex",
        "",
      );
    if (!this.provider.solMerkleTree.merkleTree)
      throw new TransactionError(
        SolMerkleTreeErrorCode.MERKLE_TREE_UNDEFINED,
        "getRootIndex",
        "Merkle tree not defined in provider.solMerkleTree",
      );

    if (this.provider.provider && this.provider.solMerkleTree.merkleTree) {
      this.merkleTreeProgram = new Program(
        IDL_MERKLE_TREE_PROGRAM,
        merkleTreeProgramId,
        // @ts-ignore
        this.provider.provider,
      );
      let root = Uint8Array.from(
        leInt2Buff(
          unstringifyBigInts(this.provider.solMerkleTree.merkleTree.root()),
          32,
        ),
      );
      let merkle_tree_account_data =
        await this.merkleTreeProgram.account.merkleTree.fetch(
          this.provider.solMerkleTree.pubkey,
          "confirmed",
        );
      // @ts-ignore: unknown type error
      merkle_tree_account_data.roots.map((x: any, index: any) => {
        if (x.toString() === root.toString()) {
          this.transactionInputs.rootIndex = index;
        }
      });

      if (this.transactionInputs.rootIndex === undefined) {
        throw new TransactionError(
          TransactionErrorCode.ROOT_NOT_FOUND,
          "getRootIndex",
          `Root index not found for root${root}`,
        );
      }
    } else {
      console.log(
        "provider not defined did not fetch rootIndex set root index to 0",
      );
      this.transactionInputs.rootIndex = 0;
    }
  }

  /**
   * @description Computes the indices in which the asset for the utxo is in the asset pubkeys array.
   * @note Using the indices the zero knowledege proof circuit enforces that only utxos containing the
   * @note assets in the asset pubkeys array are contained in the transaction.
   * @param utxos
   * @returns
   */
  // TODO: make this work for edge case of two 2 different assets plus fee asset in the same transaction
  // TODO: fix edge case of an assetpubkey being 0
  // TODO: !== !! and check non-null
  getIndices(utxos: Utxo[]): string[][][] {
    if (!this.params.assetPubkeysCircuit)
      throw new TransactionError(
        TransactionErrorCode.ASSET_PUBKEYS_UNDEFINED,
        "getIndices",
        "",
      );

    let inIndices: string[][][] = [];
    utxos.map((utxo) => {
      let tmpInIndices: string[][] = [];
      for (var a = 0; a < utxo.assets.length; a++) {
        let tmpInIndices1: string[] = [];

        for (var i = 0; i < N_ASSET_PUBKEYS; i++) {
          try {
            if (
              utxo.assetsCircuit[a].toString() ===
                this.params.assetPubkeysCircuit![i].toString() &&
              !tmpInIndices1.includes("1") &&
              this.params.assetPubkeysCircuit![i].toString() != "0"
            ) {
              tmpInIndices1.push("1");
            } else {
              tmpInIndices1.push("0");
            }
          } catch (error) {
            tmpInIndices1.push("0");
          }
        }

        tmpInIndices.push(tmpInIndices1);
      }

      inIndices.push(tmpInIndices);
    });

    // console.log(inIndices);
    return inIndices;
  }
  /**
   * @description Gets the merkle proofs for every input utxo with amounts > 0.
   * @description For input utxos with amounts == 0 it returns merkle paths with all elements = 0.
   */
  static getMerkleProofs(
    provider: Provider,
    inputUtxos: Utxo[],
  ): {
    inputMerklePathIndices: Array<string>;
    inputMerklePathElements: Array<Array<string>>;
  } {
    if (!provider.solMerkleTree)
      throw new TransactionError(
        SolMerkleTreeErrorCode.MERKLE_TREE_UNDEFINED,
        "getMerkleProofs",
        "",
      );
    if (!provider.solMerkleTree.merkleTree)
      throw new TransactionError(
        SolMerkleTreeErrorCode.MERKLE_TREE_UNDEFINED,
        "getMerkleProofs",
        "",
      );

    var inputMerklePathIndices = new Array<string>();
    var inputMerklePathElements = new Array<Array<string>>();
    // getting merkle proofs
    for (const inputUtxo of inputUtxos) {
      if (
        inputUtxo.amounts[0] > new BN(0) ||
        inputUtxo.amounts[1] > new BN(0)
      ) {
        inputUtxo.index = provider.solMerkleTree.merkleTree.indexOf(
          inputUtxo.getCommitment(),
        );

        if (inputUtxo.index || inputUtxo.index == 0) {
          if (inputUtxo.index < 0) {
            throw new TransactionError(
              TransactionErrorCode.INPUT_UTXO_NOT_INSERTED_IN_MERKLE_TREE,
              "getMerkleProofs",
              `Input commitment ${inputUtxo.getCommitment()} was not found`,
            );
          }
          inputMerklePathIndices.push(inputUtxo.index.toString());
          inputMerklePathElements.push(
            provider.solMerkleTree.merkleTree.path(inputUtxo.index)
              .pathElements,
          );
        }
      } else {
        inputMerklePathIndices.push("0");
        inputMerklePathElements.push(
          new Array<string>(provider.solMerkleTree.merkleTree.levels).fill("0"),
        );
      }
    }
    return { inputMerklePathIndices, inputMerklePathElements };
  }

  /**
   * @description
   * @returns
   */
  getTxIntegrityHash(): BN {
    if (!this.params.relayer)
      throw new TransactionError(
        TransactionErrorCode.RELAYER_UNDEFINED,
        "getTxIntegrityHash",
        "",
      );
    if (!this.params.accounts.recipient)
      throw new TransactionError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getTxIntegrityHash",
        "",
      );
    if (!this.params.accounts.recipientFee)
      throw new TransactionError(
        TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
        "getTxIntegrityHash",
        "",
      );
    if (!this.params.relayer.getRelayerFee(this.params.ataCreationFee))
      throw new TransactionError(
        TransactionErrorCode.RELAYER_FEE_UNDEFINED,
        "getTxIntegrityHash",
        "",
      );
    // Should not be computed twice because cipher texts of encrypted utxos are random
    // threfore the hash will not be consistent
    if (this.testValues && this.testValues.txIntegrityHash) {
      return this.testValues.txIntegrityHash;
    } else {
      if (!this.params.encryptedUtxos) {
        this.params.encryptedUtxos = this.encryptOutUtxos();
      }

      if (
        this.params.encryptedUtxos &&
        this.params.encryptedUtxos.length > 512
      ) {
        this.params.encryptedUtxos = this.params.encryptedUtxos.slice(0, 512);
      }
      if (
        this.params.encryptedUtxos &&
        this.testValues &&
        !this.testValues.txIntegrityHash
      ) {
        let extDataBytes = new Uint8Array([
          ...this.params.accounts.recipient?.toBytes(),
          ...this.params.accounts.recipientFee.toBytes(),
          ...this.params.relayer.accounts.relayerPubkey.toBytes(),
          ...this.params.relayer
            .getRelayerFee(this.params.ataCreationFee)
            .toArray("le", 8),
          ...this.params.encryptedUtxos,
        ]);

        const hash = keccak_256
          .create({ dkLen: 32 })
          .update(Buffer.from(extDataBytes))
          .digest();
        const txIntegrityHash: BN = new anchor.BN(hash).mod(FIELD_SIZE);
        this.testValues.txIntegrityHash = txIntegrityHash;
        return txIntegrityHash;
      } else {
        throw new TransactionError(
          TransactionErrorCode.ENCRYPTING_UTXOS_FAILED,
          "getTxIntegrityHash",
          "",
        );
      }
    }
  }

  encryptOutUtxos(encryptedUtxos?: Uint8Array) {
    let encryptedOutputs = new Array<any>();
    if (encryptedUtxos) {
      encryptedOutputs = Array.from(encryptedUtxos);
    } else if (this.params && this.params.outputUtxos) {
      this.params.outputUtxos.map((utxo) =>
        encryptedOutputs.push(utxo.encrypt()),
      );

      if (this.params.verifier.config.out == 2) {
        return new Uint8Array([
          ...encryptedOutputs[0],
          ...encryptedOutputs[1],
          ...new Array(256 - 190).fill(0),
          // this is ok because these bytes are not sent and just added for the integrity hash
          // to be consistent, if the bytes were sent to the chain use rnd bytes for padding
        ]);
      } else {
        let tmpArray = new Array<any>();
        for (var i = 0; i < this.params.verifier.config.out; i++) {
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
              this.params.verifier.config.out * 128 - tmpArray.length,
            ),
          );
        }
        return new Uint8Array([...tmpArray]);
      }
    }
  }

  // send transaction should be the same for both deposit and withdrawal
  // the function should just send the tx to the rpc or relayer respectively
  // in case there is more than one transaction to be sent to the verifier these can be sent separately
  // TODO: make optional and default no
  async getTestValues() {
    if (!this.provider)
      throw new TransactionError(
        ProviderErrorCode.PROVIDER_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.provider.provider)
      throw new TransactionError(
        ProviderErrorCode.ANCHOR_PROVIDER_UNDEFINED,
        "getTestValues",
        "Provider.provider undefined",
      );
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.params.relayer)
      throw new TransactionError(
        TransactionErrorCode.RELAYER_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.params.accounts.recipient)
      throw new TransactionError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.params.accounts.recipientFee)
      throw new TransactionError(
        TransactionErrorCode.SOL_RECIPIENT_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.params.accounts.senderFee)
      throw new TransactionError(
        TransactionErrorCode.SOL_SENDER_UNDEFINED,
        "getTestValues",
        "",
      );
    if (!this.testValues)
      throw new TransactionError(
        TransactionErrorCode.TRANSACTION_INPUTS_UNDEFINED,
        "getTestValues",
        "",
      );

    try {
      this.testValues.recipientBalancePriorTx = new BN(
        (
          await getAccount(
            this.provider.provider.connection,
            this.params.accounts.recipient,
          )
        ).amount.toString(),
      );
    } catch (e) {
      // covers the case of the recipient being a native sol address not a spl token address
      try {
        this.testValues.recipientBalancePriorTx = new BN(
          await this.provider.provider.connection.getBalance(
            this.params.accounts.recipient,
          ),
        );
      } catch (e) {}
    }

    try {
      this.testValues.recipientFeeBalancePriorTx = new BN(
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientFee,
        ),
      );
    } catch (error) {
      console.log(
        "this.testValues.recipientFeeBalancePriorTx fetch failed ",
        this.params.accounts.recipientFee,
      );
    }
    if (this.params.action === "SHIELD") {
      this.testValues.senderFeeBalancePriorTx = new BN(
        await this.provider.provider.connection.getBalance(
          this.params.relayer.accounts.relayerPubkey,
        ),
      );
    } else {
      this.testValues.senderFeeBalancePriorTx = new BN(
        await this.provider.provider.connection.getBalance(
          this.params.accounts.senderFee,
        ),
      );
    }

    this.testValues.relayerRecipientAccountBalancePriorLastTx = new BN(
      await this.provider.provider.connection.getBalance(
        this.params.relayer.accounts.relayerRecipient,
      ),
    );
  }

  static getSignerAuthorityPda(
    merkleTreeProgramId: PublicKey,
    verifierProgramId: PublicKey,
  ) {
    return PublicKey.findProgramAddressSync(
      [merkleTreeProgramId.toBytes()],
      verifierProgramId,
    )[0];
  }
  static getRegisteredVerifierPda(
    merkleTreeProgramId: PublicKey,
    verifierProgramId: PublicKey,
  ): PublicKey {
    return PublicKey.findProgramAddressSync(
      [verifierProgramId.toBytes()],
      merkleTreeProgramId,
    )[0];
  }

  async getInstructionsJson(): Promise<string[]> {
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "getInstructionsJson",
        "",
      );

    if (!this.appParams) {
      const instructions = await this.params.verifier.getInstructions(this);
      let serialized = instructions.map((ix) => JSON.stringify(ix));
      return serialized;
    } else {
      const instructions = await this.appParams.verifier.getInstructions(this);
      let serialized = instructions.map((ix: any) => JSON.stringify(ix));
      return serialized;
    }
  }

  async sendTransaction(ix: any): Promise<TransactionSignature | undefined> {
    if (this.params.action !== Action.SHIELD) {
      // TODO: replace this with (this.provider.wallet.pubkey != new relayer... this.relayer
      // then we know that an actual relayer was passed in and that it's supposed to be sent to one.
      // we cant do that tho as we'd want to add the default relayer to the provider itself.
      // so just pass in a flag here "shield, unshield, transfer" -> so devs don't have to know that it goes to a relayer.
      // send tx to relayer
      const res = await this.provider.relayer.sendTransaction(
        ix,
        this.provider,
      );
      return res;
    } else {
      if (!this.provider.provider)
        throw new TransactionError(
          ProviderErrorCode.ANCHOR_PROVIDER_UNDEFINED,
          "sendTransaction",
          "Provider.provider undefined",
        );
      if (!this.params)
        throw new TransactionError(
          TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
          "sendTransaction",
          "",
        );
      if (!this.params.relayer)
        throw new TransactionError(
          TransactionErrorCode.RELAYER_UNDEFINED,
          "sendTransaction",
          "",
        );

      if (this.transactionInputs.rootIndex === undefined) {
        throw new TransactionError(
          TransactionErrorCode.ROOT_INDEX_NOT_FETCHED,
          "sendTransaction",
          "",
        );
      }

      if (!this.remainingAccounts?.leavesPdaPubkeys) {
        throw new TransactionError(
          TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
          "sendTransaction",
          "Run await getPdaAddresses() before invoking sendTransaction",
        );
      }

      const response = await sendVersionedTransaction(ix, this.provider);
      return response;
    }
  }

  async getInstructions(): Promise<TransactionInstruction[]> {
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "getInstructions",
        "",
      );
    return await this.params.verifier.getInstructions(this);
  }

  async sendAndConfirmTransaction(): Promise<TransactionSignature> {
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "sendAndConfirmTransaction",
        "",
      );

    if (!this.provider.wallet)
      throw new TransactionError(
        TransactionErrorCode.WALLET_UNDEFINED,
        "sendAndConfirmTransaction",
        "Cannot use sendAndConfirmTransaction without wallet",
      );

    await this.getRootIndex();
    await this.getPdaAddresses();
    await this.getTestValues();

    var instructions;

    if (!this.appParams) {
      instructions = await this.params.verifier.getInstructions(this);
    } else {
      instructions = await this.appParams.verifier.getInstructions(this);
    }
    if (instructions) {
      let tx = "Something went wrong";
      for (var ix in instructions) {
        let txTmp = await this.sendTransaction(instructions[ix]);
        if (txTmp) {
          console.log("tx : ", txTmp);
          await this.provider.provider?.connection.confirmTransaction(
            txTmp,
            "confirmed",
          );
          tx = txTmp;
        } else {
          throw new TransactionError(
            TransactionErrorCode.SEND_TRANSACTION_FAILED,
            "sendAndConfirmTransaction",
            "",
          );
        }
      }
      return tx;
    } else {
      throw new TransactionError(
        TransactionErrorCode.GET_INSTRUCTIONS_FAILED,
        "sendAndConfirmTransaction",
        "",
      );
    }
  }

  // TODO: deal with this: set own payer just for that? where is this used?
  // This is used by applications not the relayer
  async closeVerifierState(): Promise<TransactionSignature> {
    if (!this.provider.wallet)
      throw new TransactionError(
        TransactionErrorCode.WALLET_UNDEFINED,
        "closeVerifierState",
        "Cannot use closeVerifierState without wallet",
      );
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "closeVerifierState",
        "",
      );
    if (!this.params.verifier.verifierProgram)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_PROGRAM_UNDEFINED,
        "closeVerifierState",
        "",
      );
    if (this.appParams) {
      const transaction = new SolanaTransaction().add(
        await this.appParams?.verifier.verifierProgram.methods
          .closeVerifierState()
          .accounts({
            ...this.params.accounts,
          })
          .instruction(),
      );

      return await this.provider.wallet!.sendAndConfirmTransaction(transaction);
    } else {
      const transaction = new SolanaTransaction().add(
        await this.params?.verifier.verifierProgram.methods
          .closeVerifierState()
          .accounts({
            ...this.params.accounts,
          })
          .instruction(),
      );

      return await this.provider.wallet!.sendAndConfirmTransaction(transaction);
    }
  }

  getPdaAddresses() {
    if (!this.transactionInputs.publicInputs)
      throw new TransactionError(
        TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "getPdaAddresses",
        "",
      );
    if (!this.params.verifier.verifierProgram)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_PROGRAM_UNDEFINED,
        "getPdaAddresses",
        "",
      );
    if (!this.params.relayer)
      throw new TransactionError(
        TransactionErrorCode.RELAYER_UNDEFINED,
        "getPdaAddresses",
        "",
      );
    if (!this.remainingAccounts)
      throw new TransactionError(
        TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
        "getPdaAddresses",
        "Remaining accounts undefined",
      );

    let nullifiers = this.transactionInputs.publicInputs.nullifiers;
    let signer = this.params.relayer.accounts.relayerPubkey;

    this.remainingAccounts.nullifierPdaPubkeys = [];
    for (var i in nullifiers) {
      this.remainingAccounts.nullifierPdaPubkeys.push({
        isSigner: false,
        isWritable: true,
        pubkey: PublicKey.findProgramAddressSync(
          [
            Uint8Array.from([...nullifiers[i]]),
            anchor.utils.bytes.utf8.encode("nf"),
          ],
          merkleTreeProgramId,
        )[0],
      });
    }

    this.remainingAccounts.leavesPdaPubkeys = [];
    for (
      var j = 0;
      j < this.transactionInputs.publicInputs.leaves.length;
      j++
    ) {
      this.remainingAccounts.leavesPdaPubkeys.push({
        isSigner: false,
        isWritable: true,
        pubkey: PublicKey.findProgramAddressSync(
          [
            Buffer.from(
              Array.from(
                this.transactionInputs.publicInputs.leaves[j][0],
              ).reverse(),
            ),
            anchor.utils.bytes.utf8.encode("leaves"),
          ],
          merkleTreeProgramId,
        )[0],
      });
    }

    if (this.appParams) {
      this.params.accounts.verifierState = PublicKey.findProgramAddressSync(
        [signer.toBytes(), anchor.utils.bytes.utf8.encode("VERIFIER_STATE")],
        this.appParams.verifier.verifierProgram.programId,
      )[0];
    } else {
      this.params.accounts.verifierState = PublicKey.findProgramAddressSync(
        [signer.toBytes(), anchor.utils.bytes.utf8.encode("VERIFIER_STATE")],
        this.params.verifier.verifierProgram.programId,
      )[0];
    }
  }

  // TODO: move in class testTransaction extends Transaction() {}
  // TODO: check why this is called encr keypair but account class
  async checkBalances(account?: Account) {
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "getPdaAddresses",
        "",
      );
    if (!this.transactionInputs.publicInputs)
      throw new TransactionError(
        TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "getPdaAddresses",
        "",
      );

    if (!this.params.accounts.senderFee) {
      throw new Error("params.accounts.senderFee undefined");
    }

    if (!this.params.accounts.recipientFee) {
      throw new Error("params.accounts.recipientFee undefined");
    }

    if (!this.params.accounts.recipient) {
      throw new Error("params.accounts.recipient undefined");
    }

    if (!this.params.accounts.recipient) {
      throw new Error("params.accounts.recipient undefined");
    }
    if (!this.testValues) {
      throw new Error("test values undefined");
    }
    if (!this.testValues.senderFeeBalancePriorTx) {
      throw new Error("senderFeeBalancePriorTx undefined");
    }

    if (!this.params.publicAmountSol) {
      throw new Error("feeAmount undefined");
    }

    if (!this.params.publicAmountSol) {
      throw new Error("feeAmount undefined");
    }

    if (!this.merkleTreeProgram) {
      throw new Error("merkleTreeProgram undefined");
    }
    this.provider.solMerkleTree;

    if (!this.provider) {
      throw new Error("provider undefined");
    }

    if (!this.provider.solMerkleTree) {
      throw new Error("provider.solMerkleTree undefined");
    }

    if (!this.params.encryptedUtxos) {
      throw new Error("params.encryptedUtxos undefined");
    }

    if (!this.params.outputUtxos) {
      throw new Error("params.outputUtxos undefined");
    }

    if (!this.provider.provider) {
      throw new Error("params.outputUtxos undefined");
    }

    if (!this.params.relayer) {
      throw new Error("params.relayer undefined");
    }

    if (!this.params.accounts.sender) {
      throw new Error("params.accounts.sender undefined");
    }
    if (!this.remainingAccounts) {
      throw new Error("remainingAccounts.nullifierPdaPubkeys undefined");
    }
    if (!this.remainingAccounts.nullifierPdaPubkeys) {
      throw new Error("remainingAccounts.nullifierPdaPubkeys undefined");
    }

    if (!this.remainingAccounts.leavesPdaPubkeys) {
      throw new Error("remainingAccounts.leavesPdaPubkeys undefined");
    }
    if (!this.testValues) {
      throw new Error("test values undefined");
    }
    if (!this.testValues.recipientFeeBalancePriorTx) {
      throw new Error("test values recipientFeeBalancePriorTx undefined");
    }

    if (!this.testValues.recipientBalancePriorTx) {
      throw new Error("test values recipientBalancePriorTx undefined");
    }

    if (!this.testValues.relayerRecipientAccountBalancePriorLastTx) {
      throw new Error(
        "test values relayerRecipientAccountBalancePriorLastTx undefined",
      );
    }
    // Checking that nullifiers were inserted
    if (new BN(this.proofInput.publicAmount).toString() === "0") {
      this.testValues.is_token = false;
    } else {
      this.testValues.is_token = true;
    }
    for (
      var i = 0;
      i < this.remainingAccounts.nullifierPdaPubkeys?.length;
      i++
    ) {
      var nullifierAccount =
        await this.provider.provider!.connection.getAccountInfo(
          this.remainingAccounts.nullifierPdaPubkeys[i].pubkey,
          {
            commitment: "confirmed",
          },
        );

      await checkRentExemption({
        account: nullifierAccount,
        connection: this.provider.provider!.connection,
      });
    }
    let leavesAccount;
    var leavesAccountData;
    // Checking that leaves were inserted
    for (var i = 0; i < this.remainingAccounts.leavesPdaPubkeys.length; i++) {
      leavesAccountData =
        await this.merkleTreeProgram.account.twoLeavesBytesPda.fetch(
          this.remainingAccounts.leavesPdaPubkeys[i].pubkey,
          "confirmed",
        );

      assert(
        leavesAccountData.nodeLeft.toString() ==
          this.transactionInputs.publicInputs.leaves[i][0].reverse().toString(),
        "left leaf not inserted correctly",
      );
      assert(
        leavesAccountData.nodeRight.toString() ==
          this.transactionInputs.publicInputs.leaves[i][1].reverse().toString(),
        "right leaf not inserted correctly",
      );
      assert(
        leavesAccountData.merkleTreePubkey.toBase58() ==
          this.provider.solMerkleTree.pubkey.toBase58(),
        "merkleTreePubkey not inserted correctly",
      );
      for (var j = 0; j < this.params.encryptedUtxos.length / 256; j++) {
        // console.log(j);

        if (
          leavesAccountData.encryptedUtxos.toString() !==
          this.params.encryptedUtxos.toString()
        ) {
          // console.log(j);
          // throw `encrypted utxo ${i} was not stored correctly`;
        }
        // console.log(
        //   `${leavesAccountData.encryptedUtxos} !== ${this.params.encryptedUtxos}`
        // );

        // assert(leavesAccountData.encryptedUtxos === this.encryptedUtxos, "encryptedUtxos not inserted correctly");
        // TODO: add for both utxos of leafpda
        let decryptedUtxo1 = Utxo.decrypt({
          poseidon: this.provider.poseidon,
          encBytes: this.params!.encryptedUtxos!,
          account: account ? account : this.params!.outputUtxos![0].account,
          index: 0, // this is just a placeholder
        });
        const utxoEqual = (utxo0: Utxo, utxo1: Utxo) => {
          assert.equal(
            utxo0.amounts[0].toString(),
            utxo1.amounts[0].toString(),
          );
          assert.equal(
            utxo0.amounts[1].toString(),
            utxo1.amounts[1].toString(),
          );
          assert.equal(utxo0.assets[0].toString(), utxo1.assets[0].toString());
          assert.equal(utxo0.assets[1].toString(), utxo1.assets[1].toString());
          assert.equal(
            utxo0.assetsCircuit[0].toString(),
            utxo1.assetsCircuit[0].toString(),
          );
          assert.equal(
            utxo0.assetsCircuit[1].toString(),
            utxo1.assetsCircuit[1].toString(),
          );
          assert.equal(
            utxo0.instructionType.toString(),
            utxo1.instructionType.toString(),
          );
          assert.equal(utxo0.poolType.toString(), utxo1.poolType.toString());
          assert.equal(
            utxo0.verifierAddress.toString(),
            utxo1.verifierAddress.toString(),
          );
          assert.equal(
            utxo0.verifierAddressCircuit.toString(),
            utxo1.verifierAddressCircuit.toString(),
          );
        };
        // console.log("decryptedUtxo ", decryptedUtxo1);
        // console.log("this.params.outputUtxos[0] ", this.params.outputUtxos[0]);
        if (decryptedUtxo1 !== null) {
          utxoEqual(decryptedUtxo1, this.params.outputUtxos[0]);
        } else {
          console.log("Could not decrypt any utxo probably a withdrawal.");
        }
      }
    }

    console.log(
      `mode ${this.params.action}, this.testValues.is_token ${this.testValues.is_token}`,
    );

    try {
      const merkleTreeAfterUpdate =
        await this.merkleTreeProgram.account.merkleTree.fetch(
          MERKLE_TREE_KEY,
          "confirmed",
        );
      console.log(
        "Number(merkleTreeAfterUpdate.nextQueuedIndex) ",
        Number(merkleTreeAfterUpdate.nextQueuedIndex),
      );
      leavesAccountData =
        await this.merkleTreeProgram.account.twoLeavesBytesPda.fetch(
          this.remainingAccounts.leavesPdaPubkeys[0].pubkey,
          "confirmed",
        );
      console.log(
        `${Number(leavesAccountData.leftLeafIndex)} + ${
          this.remainingAccounts.leavesPdaPubkeys.length * 2
        }`,
      );

      assert.equal(
        Number(merkleTreeAfterUpdate.nextQueuedIndex),
        Number(leavesAccountData.leftLeafIndex) +
          this.remainingAccounts.leavesPdaPubkeys.length * 2,
      );
    } catch (e) {
      console.log("preInsertedLeavesIndex: ", e);
    }
    var nrInstructions;
    if (this.appParams) {
      nrInstructions = this.appParams.verifier.instructions?.length;
    } else if (this.params) {
      nrInstructions = this.params.verifier.instructions?.length;
    } else {
      throw new Error("No params provided.");
    }
    console.log("nrInstructions ", nrInstructions);

    if (this.params.action == "SHIELD" && this.testValues.is_token == false) {
      var recipientFeeAccountBalance =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientFee,
          "confirmed",
        );
      console.log(
        "testValues.recipientFeeBalancePriorTx: ",
        this.testValues.recipientFeeBalancePriorTx,
      );

      var senderFeeAccountBalance =
        await this.provider.provider.connection.getBalance(
          this.params.relayer.accounts.relayerPubkey,
          "confirmed",
        );
      assert(
        recipientFeeAccountBalance ==
          Number(this.testValues.recipientFeeBalancePriorTx) +
            Number(this.params.publicAmountSol),
      );
      console.log(
        `${new BN(this.testValues.senderFeeBalancePriorTx)
          .sub(this.params.publicAmountSol)
          .sub(new BN(5000 * nrInstructions))
          .toString()} == ${senderFeeAccountBalance}`,
      );
      assert(
        new BN(this.testValues.senderFeeBalancePriorTx)
          .sub(this.params.publicAmountSol)
          .sub(new BN(5000 * nrInstructions))
          .toString() == senderFeeAccountBalance.toString(),
      );
    } else if (
      this.params.action == "SHIELD" &&
      this.testValues.is_token == true
    ) {
      console.log("SHIELD and token");

      var recipientAccount = await getAccount(
        this.provider.provider.connection,
        this.params.accounts.recipient,
      );
      var recipientFeeAccountBalance =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientFee,
        );

      // console.log(`Balance now ${senderAccount.amount} balance beginning ${senderAccountBalancePriorLastTx}`)
      // assert(senderAccount.lamports == (I64(senderAccountBalancePriorLastTx) - I64.readLE(this.extAmount, 0)).toString(), "amount not transferred correctly");

      console.log(
        `Balance now ${recipientAccount.amount} balance beginning ${this.testValues.recipientBalancePriorTx}`,
      );
      console.log(
        `Balance now ${recipientAccount.amount} balance beginning ${
          Number(this.testValues.recipientBalancePriorTx) +
          Number(this.params.publicAmountSpl)
        }`,
      );
      assert(
        recipientAccount.amount.toString() ===
          (
            Number(this.testValues.recipientBalancePriorTx) +
            Number(this.params.publicAmountSpl)
          ).toString(),
        "amount not transferred correctly",
      );
      console.log(
        `Blanace now ${recipientFeeAccountBalance} ${
          Number(this.testValues.recipientFeeBalancePriorTx) +
          Number(this.params.publicAmountSol)
        }`,
      );
      console.log("fee amount: ", this.params.publicAmountSol);
      console.log(
        "fee amount from inputs. ",
        new anchor.BN(
          this.transactionInputs.publicInputs.feeAmount.slice(24, 32),
        ).toString(),
      );
      console.log(
        "pub amount from inputs. ",
        new anchor.BN(
          this.transactionInputs.publicInputs.publicAmount.slice(24, 32),
        ).toString(),
      );

      var senderFeeAccountBalance =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.senderFee,
          "confirmed",
        );

      assert(
        recipientFeeAccountBalance ==
          Number(this.testValues.recipientFeeBalancePriorTx) +
            Number(this.params.publicAmountSol),
      );
      console.log(
        `${new BN(this.testValues.senderFeeBalancePriorTx)
          .sub(this.params.publicAmountSol)
          .sub(new BN(5000 * nrInstructions))
          .toString()} == ${senderFeeAccountBalance}`,
      );
      assert(
        new BN(this.testValues.senderFeeBalancePriorTx)
          .sub(this.params.publicAmountSol)
          .sub(new BN(5000 * nrInstructions))
          .toString() == senderFeeAccountBalance.toString(),
      );
    } else if (
      this.params.action == "UNSHIELD" &&
      this.testValues.is_token == false
    ) {
      var relayerAccount = await this.provider.provider.connection.getBalance(
        this.params.relayer.accounts.relayerRecipient,
        "confirmed",
      );

      var recipientFeeAccount =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientFee,
          "confirmed",
        );

      console.log(
        "testValues.relayerRecipientAccountBalancePriorLastTx ",
        this.testValues.relayerRecipientAccountBalancePriorLastTx,
      );
      console.log(
        `relayerFeeAccount ${new anchor.BN(relayerAccount)
          .sub(this.params.relayer.getRelayerFee(this.params.ataCreationFee))
          .toString()} == ${new anchor.BN(
          this.testValues.relayerRecipientAccountBalancePriorLastTx,
        )}`,
      );
      console.log(
        `recipientFeeAccount ${new anchor.BN(recipientFeeAccount)
          .add(
            new anchor.BN(
              this.params.relayer
                .getRelayerFee(this.params.ataCreationFee)
                .toString(),
            ),
          )
          .toString()}  == ${new anchor.BN(
          this.testValues.recipientFeeBalancePriorTx,
        )
          .sub(this.params.publicAmountSol?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );

      assert.equal(
        new anchor.BN(recipientFeeAccount)
          .add(
            new anchor.BN(
              this.params.relayer
                .getRelayerFee(this.params.ataCreationFee)
                .toString(),
            ),
          )
          .toString(),
        new anchor.BN(this.testValues.recipientFeeBalancePriorTx)
          .sub(this.params.publicAmountSol?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString(),
      );
      // console.log(`this.params.relayer.relayerFee ${this.params.relayer.relayerFee} new anchor.BN(relayerAccount) ${new anchor.BN(relayerAccount)}`);
      assert.equal(
        new anchor.BN(relayerAccount)
          .sub(this.params.relayer.getRelayerFee(this.params.ataCreationFee))
          .toString(),
        this.testValues.relayerRecipientAccountBalancePriorLastTx?.toString(),
      );
    } else if (
      this.params.action == "UNSHIELD" &&
      this.testValues.is_token == true
    ) {
      var senderAccount = await getAccount(
        this.provider.provider.connection,
        this.params.accounts.sender,
      );

      var recipientAccount = await getAccount(
        this.provider.provider.connection,
        this.params.accounts.recipient,
      );

      // assert(senderAccount.amount == ((I64(Number(senderAccountBalancePriorLastTx)).add(I64.readLE(this.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");
      console.log(
        "this.testValues.recipientBalancePriorTx ",
        this.testValues.recipientBalancePriorTx,
      );
      console.log("this.params.publicAmountSpl ", this.params.publicAmountSpl);
      console.log(
        "this.params.publicAmountSpl ",
        this.params.publicAmountSpl?.sub(FIELD_SIZE).mod(FIELD_SIZE),
      );

      console.log(
        `${recipientAccount.amount}, ${new anchor.BN(
          this.testValues.recipientBalancePriorTx,
        )
          .sub(this.params.publicAmountSpl?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );
      assert.equal(
        recipientAccount.amount.toString(),
        new anchor.BN(this.testValues.recipientBalancePriorTx)
          .sub(this.params.publicAmountSpl?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString(),
        "amount not transferred correctly",
      );

      var relayerAccount = await this.provider.provider.connection.getBalance(
        this.params.relayer.accounts.relayerRecipient,
        "confirmed",
      );

      var recipientFeeAccount =
        await this.provider.provider.connection.getBalance(
          this.params.accounts.recipientFee,
          "confirmed",
        );

      // console.log("relayerAccount ", relayerAccount);
      // console.log("this.params.relayer.relayerFee: ", this.params.relayer.getRelayerFee);
      console.log(
        "testValues.relayerRecipientAccountBalancePriorLastTx ",
        this.testValues.relayerRecipientAccountBalancePriorLastTx,
      );
      console.log(
        `relayerFeeAccount ${new anchor.BN(relayerAccount)
          .sub(this.params.relayer.getRelayerFee(this.params.ataCreationFee))
          .toString()} == ${new anchor.BN(
          this.testValues.relayerRecipientAccountBalancePriorLastTx,
        )}`,
      );

      console.log(
        `recipientFeeAccount ${new anchor.BN(recipientFeeAccount)
          .add(
            new anchor.BN(
              this.params.relayer
                .getRelayerFee(this.params.ataCreationFee)
                .toString(),
            ),
          )
          .toString()}  == ${new anchor.BN(
          this.testValues.recipientFeeBalancePriorTx,
        )
          .sub(this.params.publicAmountSol?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString()}`,
      );

      assert.equal(
        new anchor.BN(recipientFeeAccount)
          .add(
            new anchor.BN(
              this.params.relayer
                .getRelayerFee(this.params.ataCreationFee)
                .toString(),
            ),
          )
          .toString(),
        new anchor.BN(this.testValues.recipientFeeBalancePriorTx)
          .sub(this.params.publicAmountSol?.sub(FIELD_SIZE).mod(FIELD_SIZE))
          .toString(),
      );

      assert.equal(
        new anchor.BN(relayerAccount)
          .sub(this.params.relayer.getRelayerFee(this.params.ataCreationFee))
          // .add(new anchor.BN("5000"))
          .toString(),
        this.testValues.relayerRecipientAccountBalancePriorLastTx?.toString(),
      );
    } else if (this.params.action === Action.TRANSFER) {
      console.log("balance check for transfer not implemented");
    } else {
      throw Error("mode not supplied");
    }
  }

  // TODO: use higher entropy rnds
  shuffleUtxos(utxos: Utxo[]) {
    if (this.shuffleEnabled) {
      console.log("shuffling utxos");
    } else {
      console.log("shuffle disabled");
      return;
    }
    let currentIndex: number = utxos.length;
    let randomIndex: number;

    // While there remain elements to shuffle...
    while (0 !== currentIndex) {
      // Pick a remaining element...
      randomIndex = Math.floor(Math.random() * currentIndex);
      currentIndex--;

      // And swap it with the current element.
      [utxos[currentIndex], utxos[randomIndex]] = [
        utxos[randomIndex],
        utxos[currentIndex],
      ];
    }

    return utxos;
  }

  // also converts lE to BE
  static async parseProofToBytesArray(data: any) {
    var mydata = JSON.parse(data.toString());

    for (var i in mydata) {
      if (i == "pi_a" || i == "pi_c") {
        for (var j in mydata[i]) {
          mydata[i][j] = Array.from(
            leInt2Buff(unstringifyBigInts(mydata[i][j]), 32),
          ).reverse();
        }
      } else if (i == "pi_b") {
        for (var j in mydata[i]) {
          for (var z in mydata[i][j]) {
            mydata[i][j][z] = Array.from(
              leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32),
            );
          }
        }
      }
    }
    return {
      proofA: [mydata.pi_a[0], mydata.pi_a[1]].flat(),
      proofB: [
        mydata.pi_b[0].flat().reverse(),
        mydata.pi_b[1].flat().reverse(),
      ].flat(),
      proofC: [mydata.pi_c[0], mydata.pi_c[1]].flat(),
    };
  }

  static getTokenAuthority(): PublicKey {
    return PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("spl")],
      merkleTreeProgramId,
    )[0];
  }
}
