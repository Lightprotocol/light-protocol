import { BN, Idl, Provider } from "@coral-xyz/anchor";
import { ParsedMessageAccount, PublicKey } from "@solana/web3.js";
import { Relayer } from "../relayer";
import { Action } from "../transaction";
import { Utxo } from "../utxo";
import { Verifier } from "../verifiers";

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

export type IndexedTransaction = {
  blockTime: number;
  signer: PublicKey;
  signature: string;
  accounts: ParsedMessageAccount[];
  to: PublicKey;
  from: PublicKey;
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

export type UserIndexedTransaction = IndexedTransaction & {
  inSpentUtxos: Utxo[];
  outSpentUtxos: Utxo[];
};
