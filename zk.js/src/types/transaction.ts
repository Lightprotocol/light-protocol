import { BN, Idl, Provider } from "@coral-xyz/anchor";
import {
  BlockhashWithExpiryBlockHeight,
  PublicKey,
  TransactionInstruction,
  TransactionSignature,
  VersionedTransaction,
  VersionedTransactionResponse,
} from "@solana/web3.js";
import { Rpc } from "../rpc";
import { PlaceHolderTData, ProgramUtxo, Utxo } from "../utxo";

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
  rpc?: Rpc;
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
  rpcRecipientSol: PublicKey;
  type: Action;
  changeSolAmount: string;
  publicAmountSol: string;
  publicAmountSpl: string;
  encryptedUtxos: Buffer;
  leaves: number[][];
  firstLeafIndex: string;
  nullifiers: number[][];
  rpcFee: string;
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
  rpcRecipientSol: string;
  type: Action;
  changeSolAmount: string;
  publicAmountSol: string;
  publicAmountSpl: string;
  encryptedUtxos: number[];
  leaves: number[][];
  firstLeafIndex: string;
  nullifiers: number[][];
  rpcFee: string;
  message: number[];
};

// Rpc internal type
export type RpcIndexedTransaction = {
  transaction: ParsedIndexedTransaction;
  IDs: string[];
  merkleTreePublicKey: string;
};

// RPC response type
export type RpcIndexedTransactionResponse = {
  transaction: ParsedIndexedTransaction;
  merkleProofs: string[][];
  leavesIndexes: number[];
};

// User internal type
export type UserIndexedTransaction = ParsedIndexedTransaction & {
  inSpentUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
  outSpentUtxos: (Utxo | ProgramUtxo<PlaceHolderTData>)[];
};

export enum Action {
  COMPRESS = "COMPRESS",
  TRANSFER = "TRANSFER",
  DECOMPRESS = "DECOMPRESS",
}

export type PublicInputs = {
  root: Array<number>;
  publicAmountSpl: Array<number>;
  txIntegrityHash: Array<number>;
  publicAmountSol: Array<number>;
  publicMintPubkey: Array<number>;
  publicNullifier: Array<Array<number>>;
  publicOutUtxoHash: Array<Array<number>>;
  // only for app verifiers
  transactionHash?: Array<number>;
  checkedParams?: Array<Array<number>>;
  publicAppVerifier?: Array<number>;
};
