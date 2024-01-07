// TODO: add History type
import { BN, Idl } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

// top-level abstraction for TokenBalances
export type Balance = Map<string, TokenBalance>; // key: mint

export type SerializedBalance = SerializedTokenBalance[];

/** Each mint can have public/private UTXOs */
export type TokenBalance = {
  splAmount: BN;
  lamports: BN;
  tokenData: TokenData;
  utxos: DefaultUtxo[];
  publicUtxos: PublicUtxo[];
  /** The slot number at which the token balance was last updated, corresponding to the slot of the latest UTXO event. */
  contextSlot: Slot;
};

/** Serialized tokenBalance */
export type SerializedTokenBalance = {
  mint: string;
  utxos: { utxo: string; index?: number }[];
  publicUtxos: { utxo: string; index?: number }[];
  contextSlot: Slot;
};

/** Context for Tokens. Synced with on-chain registry. */
export type TokenData = {
  mint: PublicKey;
  symbol: string;
  decimals: BN;
  isNft: boolean;
  isNative: boolean;
  enforcePublic: boolean; // TODO: do we need this? default: false
  // TODO: would want potentially other things such as authority / creator in here
  // anything special for cNFTs required here?
};

/** RPC context slot */
export type Slot = number;

export type SyncOptions = {
  /** Mint or ProgramIds */
  ids?: PublicKey[];
  /** minimum slot to fetch from. This is useful for iterative scanning
   * ignores all UTXOs inserted before specified slot.
   */
  minSlot?: Slot;
  /** RPC can either return public or encrypted data. default = encrypted */
  shouldSyncPublic?: boolean;
};

export type UtxoNewNew = {
  publicKey: string;
  amounts: BN[];
  assets: PublicKey[];
  assetsCircuit: string[];
  blinding: string;
  poolType: string;
  utxoHash: string;
  transactionVersion: string;
  verifierAddress: PublicKey;
  verifierAddressCircuit: string;
  isFillingUtxo: boolean;
  nullifier: string;
  merkleTreeLeafIndex: number;
  merkleProof: string[];
  utxoDataHash: string;
  isPublic?: boolean; // added
};

export type ProgramUtxo = {
  utxo: UtxoNewNew;
  pspId: PublicKey;
  pspIdl: Idl;
  includeUtxoData: boolean;
  utxoData: any; // TODO: make depend on idl // could this be used as metadata field?
  utxoName: string;
};

// default: !isPublic
export type DefaultUtxo = UtxoNewNew & { isPublic: false };

// Enforce isPublic
export type PublicUtxo = UtxoNewNew & { isPublic: true };

export type DefaultProgramUtxo = ProgramUtxo & {
  utxo: DefaultUtxo;
};

export type PublicProgramUtxo = ProgramUtxo & {
  utxo: PublicUtxo;
};

// Akin to all PDAs belonging to a Program
// This is the top-level abstraction for Program state
// Dapps want to control program-specific state
// Wallets don't want to know about ProgramState at all.
export type ProgramState = {
  owner: PublicKey; // this can be public program or psp
  ownerIdl: Idl;
  utxos: DefaultProgramUtxo[]; /// these can be unique or not unique UTXOs (accounts)
  publicUtxos: PublicProgramUtxo[];
};
