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
import { Action } from "../transaction";
import { Utxo } from "../utxo";

export type AppUtxoConfig = {
  verifierAddress: PublicKey;
  appData?: any;
  appDataHash?: BN;
  includeAppData?: boolean;
  idl: Idl;
};

export type PrioritizationFee = bigint;

export type LightTransaction = VersionedTransaction[];
export type LightTransactionResponse = VersionedTransactionResponse[];
export type LightTransactionSignature = TransactionSignature[];

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

export type RelayerRelayPayload = {
  instructions: TransactionInstruction[];
  prioritizationFee?: string;
};

// TODO: we should replace this for the Poseidon release
export type ActionResponse = {
  txHash: { signatures: TransactionSignature };
  response: string;
};
export type ActionResponseMulti = {
  txHash: { signatures: TransactionSignature[] };
  response: string;
};

export type SignaturesWithBlockhashInfo = {
  signatures: TransactionSignature[];
  blockhashInfo: BlockhashWithExpiryBlockHeight;
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
  encryptedUtxos: Buffer | any[];
  leaves: number[][];
  firstLeafIndex: string;
  nullifiers: BN[];
  relayerFee: string;
  message: Buffer;
};
export type ParsedIndexedTransaction = {
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
  changeSolAmount: BN;
  publicAmountSol: BN;
  publicAmountSpl: BN;
  encryptedUtxos: Buffer | any[];
  leaves: number[][];
  firstLeafIndex: BN;
  nullifiers: BN[];
  relayerFee: BN;
  message: Buffer;
};

export type UserIndexedTransaction = ParsedIndexedTransaction & {
  inSpentUtxos: Utxo[];
  outSpentUtxos: Utxo[];
};
