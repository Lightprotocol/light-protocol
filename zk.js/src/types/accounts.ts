import { PublicKey } from "@solana/web3.js";

export type lightAccounts = {
  senderSpl: PublicKey;
  recipientSpl?: PublicKey;
  senderSol: PublicKey;
  recipientSol?: PublicKey;
  tokenAuthority: PublicKey;
  systemProgramId: PublicKey;
  merkleTreeSet: PublicKey;
  tokenProgram: PublicKey;
  registeredVerifierPda: PublicKey;
  authority: PublicKey;
  signingAddress: PublicKey;
  programMerkleTree: PublicKey;
  logWrapper: PublicKey;
  rpcRecipientSol: PublicKey;
  verifierProgram?: PublicKey;
  verifierState?: PublicKey;
};

export type remainingAccount = {
  isSigner: boolean;
  isWritable: boolean;
  pubkey: PublicKey;
};
