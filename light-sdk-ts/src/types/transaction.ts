import { BN, Provider } from "@coral-xyz/anchor";
import { ParsedMessageAccount, PublicKey } from "@solana/web3.js";
import { Relayer } from "relayer";
import { Action } from "../transaction";
import { Utxo } from "utxo";
import { Verifier } from "verifiers";

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

export type historyTransaction = {
  blockTime: number;
  signer: PublicKey;
  signature: string;
  accounts: ParsedMessageAccount[];
  to: PublicKey;
  from: PublicKey;
  type: Action;
  amount: BN;
  amountSol: BN;
  amountSpl: BN;
  commitment: string;
  encryptedUtxos: Buffer | any[];
  leaves: BN[];
  nullifiers: BN[];
  relayerFee: BN;
};
