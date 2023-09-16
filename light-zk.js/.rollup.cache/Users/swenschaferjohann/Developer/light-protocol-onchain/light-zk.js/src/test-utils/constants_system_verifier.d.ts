/// <reference types="bn.js" />
import { Keypair, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
export declare const ENCRYPTION_KEYPAIR: {
    PublicKey: Uint8Array;
    secretKey: Uint8Array;
};
export declare const USER_TOKEN_ACCOUNT: Keypair;
export declare const RECIPIENT_TOKEN_ACCOUNT: Keypair;
export declare var KEYPAIR_PRIVKEY: BN;
export declare const MINT_PRIVATE_KEY: Uint8Array;
export declare const MINT: PublicKey;
export declare const PRIVATE_KEY: number[];
export declare const PRIVATE_KEY_RELAYER: number[];
export declare const MERKLE_TREE_INIT_AUTHORITY: number[];
export declare const ADMIN_AUTH_KEY: PublicKey;
export declare const ADMIN_AUTH_KEYPAIR: Keypair;
export declare const RELAYER_RECIPIENT_KEYPAIR: Keypair;
export declare const userTokenAccount: PublicKey;
export declare const recipientTokenAccount: PublicKey;
//# sourceMappingURL=constants_system_verifier.d.ts.map