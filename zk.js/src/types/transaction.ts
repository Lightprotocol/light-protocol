import { BN, Idl, Provider } from "@coral-xyz/anchor";
import {
  BlockhashWithExpiryBlockHeight,
  PublicKey,
  TransactionInstruction,
  TransactionSignature,
  VersionedTransaction,
  VersionedTransactionResponse,
} from "@solana/web3.js";
import { Relayer } from "../relayer";
import { Utxo } from "../utxo";

export type PrioritizationFee = bigint;

export type LightTransaction = VersionedTransaction[];
export type LightTransactionResponse = VersionedTransactionResponse[];
export type LightTransactionSignature = TransactionSignature[];

export type RelayerRelayPayload = {
  instructions: TransactionInstruction[];
  prioritizationFee?: string;
};

// TODO: create unified interface for consumers
/** these are placeholder types until we unify the external response interface */
export type ActionResponseMulti = {
  txHash: { signatures: TransactionSignature[] };
  response: string;
};

export type SignaturesWithBlockhashInfo = {
  signatures: TransactionSignature[];
  blockhashInfo: BlockhashWithExpiryBlockHeight;
};

export type AppUtxoConfig = {
  verifierAddress: PublicKey;
  appData?: any;
  appDataHash?: BN;
  includeAppData?: boolean;
  idl: Idl;
};

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

export type IndexedTransaction = {
  blockTime: number;
  signer: PublicKey;
  signature: string;
  to: PublicKey;
  from: PublicKey;
  toSpl: PublicKey;
  fromSpl: PublicKey;
  verifier: PublicKey;
  relayerRecipientSol: PublicKey;
  type: Action;
  changeSolAmount: string;
  publicAmountSol: string;
  publicAmountSpl: string;
  encryptedUtxos: Buffer;
  leaves: number[][];
  firstLeafIndex: string;
  nullifiers: number[][];
  relayerFee: string;
  message: Buffer;
};

export type ParsedIndexedTransaction = {
  blockTime: number;
  signer: string;
  signature: string;
  to: string;
  from: string;
  toSpl: string;
  fromSpl: string;
  verifier: string;
  relayerRecipientSol: string;
  type: Action;
  changeSolAmount: string;
  publicAmountSol: string;
  publicAmountSpl: string;
  encryptedUtxos: number[];
  leaves: number[][];
  firstLeafIndex: string;
  nullifiers: number[][];
  relayerFee: string;
  message: number[];
};

// Relayer internal type
export type RelayerIndexedTransaction = {
  transaction: ParsedIndexedTransaction;
  IDs: string[];
  merkleTreePublicKey: string;
};

// RPC response type
export type RpcIndexedTransaction = {
  transaction: ParsedIndexedTransaction;
  merkleProofs: string[][];
  leavesIndexes: number[];
};

// User internal type
export type UserIndexedTransaction = ParsedIndexedTransaction & {
  inSpentUtxos: Utxo[];
  outSpentUtxos: Utxo[];
};

export enum Action {
  SHIELD = "SHIELD",
  TRANSFER = "TRANSFER",
  UNSHIELD = "UNSHIELD",
}

export type PublicInputs = {
  root: Array<number>;
  publicAmountSpl: Array<number>;
  txIntegrityHash: Array<number>;
  publicAmountSol: Array<number>;
  publicMintPubkey: Array<number>;
  inputNullifier: Array<Array<number>>;
  outputCommitment: Array<Array<number>>;
  // only for app verifiers
  transactionHash?: Array<number>;
  checkedParams?: Array<Array<number>>;
  publicAppVerifier?: Array<number>;
};
