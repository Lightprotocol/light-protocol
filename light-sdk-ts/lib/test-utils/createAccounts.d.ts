import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair } from "@solana/web3.js";
import { Keypair as ShieldedKeypair } from "../keypair";
import { Program } from "@coral-xyz/anchor";
export declare const newAccountWithLamports: (connection: Connection, account?: any, lamports?: number) => Promise<any>;
export declare const newAddressWithLamports: (connection: Connection, address?: anchor.web3.PublicKey, lamports?: number) => Promise<anchor.web3.PublicKey>;
export declare const newProgramOwnedAccount: ({ connection, owner, lamports, }: {
    connection: Connection;
    owner: Program;
    lamports: Number;
}) => Promise<anchor.web3.Account>;
export declare function newAccountWithTokens({ connection, MINT, ADMIN_AUTH_KEYPAIR, userAccount, amount, }: {
    connection: any;
    MINT: any;
    ADMIN_AUTH_KEYPAIR: any;
    userAccount: any;
    amount: any;
}): Promise<any>;
export declare function createMintWrapper({ authorityKeypair, mintKeypair, nft, decimals, connection, }: {
    authorityKeypair: Keypair;
    mintKeypair?: Keypair;
    nft?: Boolean;
    decimals?: number;
    connection: Connection;
}): Promise<anchor.web3.PublicKey | undefined>;
export declare function createTestAccounts(connection: Connection): Promise<{
    POSEIDON: any;
    KEYPAIR: ShieldedKeypair;
    RELAYER_RECIPIENT: anchor.web3.PublicKey;
}>;
