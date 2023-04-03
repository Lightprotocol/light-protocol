import { PublicKey } from "@solana/web3.js";

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
