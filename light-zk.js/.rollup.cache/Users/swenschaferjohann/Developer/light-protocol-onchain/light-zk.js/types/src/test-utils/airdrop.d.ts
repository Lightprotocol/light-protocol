import { Connection, PublicKey } from "@solana/web3.js";
import { Provider } from "../index";
export declare function airdropShieldedSol({ provider, amount, seed, recipientPublicKey, }: {
    provider?: Provider;
    amount: number;
    seed?: string;
    recipientPublicKey?: string;
}): Promise<{
    txHash: any;
    response: string;
}>;
export declare function airdropSol({ connection, lamports, recipientPublicKey, }: {
    connection: Connection;
    lamports: number;
    recipientPublicKey: PublicKey;
}): Promise<string>;
/**
 * airdrops shielded spl tokens from ADMIN_AUTH_KEYPAIR to the user specified by seed if aes encrypted desired, or by recipient pubkey if nacl box encrypted (will be in utxoInbox then)
 * @param param0
 * @returns
 */
export declare function airdropShieldedMINTSpl({ provider, amount, seed, recipientPublicKey, }: {
    provider?: Provider;
    amount: number;
    seed?: string;
    recipientPublicKey?: string;
}): Promise<{
    txHash: any;
    response: string;
}>;
export declare function airdropSplToAssociatedTokenAccount(connection: Connection, lamports: number, recipient: PublicKey): Promise<string>;
//# sourceMappingURL=airdrop.d.ts.map