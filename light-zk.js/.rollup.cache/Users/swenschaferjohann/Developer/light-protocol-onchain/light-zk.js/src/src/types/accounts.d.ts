import { PublicKey } from "@solana/web3.js";
export type lightAccounts = {
    senderSpl?: PublicKey;
    recipientSpl?: PublicKey;
    senderSol?: PublicKey;
    recipientSol?: PublicKey;
    verifierState?: PublicKey;
    tokenAuthority?: PublicKey;
    systemProgramId: PublicKey;
    eventMerkleTree?: PublicKey;
    transactionMerkleTree: PublicKey;
    tokenProgram: PublicKey;
    registeredVerifierPda: PublicKey;
    authority: PublicKey;
    signingAddress?: PublicKey;
    programMerkleTree: PublicKey;
    logWrapper: PublicKey;
    verifierProgram?: PublicKey;
};
export type remainingAccount = {
    isSigner: boolean;
    isWritable: boolean;
    pubkey: PublicKey;
};
//# sourceMappingURL=accounts.d.ts.map