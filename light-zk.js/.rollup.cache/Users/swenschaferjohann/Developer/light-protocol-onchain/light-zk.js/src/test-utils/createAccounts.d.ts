/// <reference types="bn.js" />
import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { Account } from "../account";
import { Program } from "@coral-xyz/anchor";
export declare const newAccountWithLamports: (connection: Connection, account?: anchor.web3.Keypair, lamports?: number) => Promise<anchor.web3.Keypair>;
export declare const newAddressWithLamports: (connection: Connection, address?: anchor.web3.PublicKey, lamports?: number) => Promise<anchor.web3.PublicKey>;
export declare const newProgramOwnedAccount: ({ connection, owner, }: {
    connection: Connection;
    owner: Program;
    lamports: Number;
}) => Promise<anchor.web3.Account>;
export declare function newAccountWithTokens({ connection, MINT, ADMIN_AUTH_KEYPAIR, userAccount, amount, }: {
    connection: Connection;
    MINT: PublicKey;
    ADMIN_AUTH_KEYPAIR: Keypair;
    userAccount: Keypair;
    amount: BN;
}): Promise<any>;
export declare function createMintWrapper({ authorityKeypair, mintKeypair, nft, decimals, connection, }: {
    authorityKeypair: Keypair;
    mintKeypair?: Keypair;
    nft?: Boolean;
    decimals?: number;
    connection: Connection;
}): Promise<anchor.web3.PublicKey | undefined>;
export declare function createTestAccounts(connection: Connection, userTokenAccount?: PublicKey): Promise<{
    POSEIDON: any;
    KEYPAIR: Account;
    RELAYER_RECIPIENT: anchor.web3.PublicKey;
}>;
//# sourceMappingURL=createAccounts.d.ts.map