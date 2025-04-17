import { Commitment, PublicKey } from '@solana/web3.js';
import { unpackAccount } from '@solana/spl-token';
import { CompressedTokenProgram } from '../program';
import { Rpc } from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import { Buffer } from 'buffer';

/**
 * Check if the token pool info is initialized and has a balance.
 * @param mint The mint of the token pool
 * @param tokenPoolInfo The token pool info
 * @returns True if the token pool info is initialized and has a balance
 */
export function checkTokenPoolInfo(
    tokenPoolInfo: TokenPoolInfo,
    mint: PublicKey,
): boolean {
    if (!tokenPoolInfo.mint.equals(mint)) {
        throw new Error(`TokenPool mint does not match the provided mint.`);
    }

    if (!tokenPoolInfo.isInitialized) {
        throw new Error(
            `TokenPool is not initialized. Please create a compressed token pool for mint: ${mint.toBase58()} via createTokenPool().`,
        );
    }
    return true;
}

/**
 * Get the token pool infos for a given mint.
 * @param rpc         The RPC client
 * @param mint        The mint of the token pool
 * @param commitment  The commitment to use
 *
 * @returns The token pool infos
 */
export async function getTokenPoolInfos(
    rpc: Rpc,
    mint: PublicKey,
    commitment?: Commitment,
): Promise<TokenPoolInfo[]> {
    const addresses = Array.from({ length: 5 }, (_, i) =>
        deriveTokenPoolPdaWithBump(mint, i),
    );

    const accountInfos = await rpc.getMultipleAccountsInfo(
        addresses,
        commitment,
    );

    if (accountInfos[0] === null) {
        throw new Error(
            `TokenPool not found. Please create a compressed token pool for mint: ${mint.toBase58()} via createTokenPool().`,
        );
    }

    const parsedInfos = addresses.map((address, i) =>
        accountInfos[i]
            ? unpackAccount(address, accountInfos[i], accountInfos[i].owner)
            : null,
    );

    const tokenProgram = accountInfos[0]!.owner;
    return parsedInfos.map((parsedInfo, i) => {
        if (!parsedInfo) {
            return {
                mint,
                tokenPoolPda: addresses[i],
                tokenProgram,
                activity: undefined,
                balance: new BN(0),
                isInitialized: false,
            };
        }

        return {
            mint,
            tokenPoolPda: parsedInfo.address,
            tokenProgram,
            activity: undefined,
            balance: new BN(parsedInfo.amount.toString()),
            isInitialized: true,
        };
    });
}

export type TokenPoolActivity = {
    signature: string;
    amount: BN;
    action: Action;
};

/**
 * Derive the token pool pda with bump.
 *
 * @param mint The mint of the token pool
 * @param bump Bump. starts at 0. The Protocol supports up to 4 bumps aka token
 * pools per mint.
 *
 * @returns The token pool pda
 */
export function deriveTokenPoolPdaWithBump(
    mint: PublicKey,
    bump: number,
): PublicKey {
    let seeds: Buffer[] = [];
    if (bump === 0) {
        seeds = [Buffer.from('pool'), mint.toBuffer()]; // legacy, 1st
    } else {
        seeds = [Buffer.from('pool'), mint.toBuffer(), Buffer.from([bump])];
    }
    const [address, _] = PublicKey.findProgramAddressSync(
        seeds,
        CompressedTokenProgram.programId,
    );
    return address;
}

/**
 * Token pool pda info.
 */
export type TokenPoolInfo = {
    /**
     * The mint of the token pool
     */
    mint: PublicKey;
    /**
     * The token pool address
     */
    tokenPoolPda: PublicKey;
    /**
     * The token program of the token pool
     */
    tokenProgram: PublicKey;
    /**
     * count of txs and volume in the past 60 seconds.
     */
    activity?: {
        txs: number;
        amountAdded: BN;
        amountRemoved: BN;
    };
    /**
     * Whether the token pool is initialized
     */
    isInitialized: boolean;
    /**
     * The balance of the token pool
     */
    balance: BN;
};

/**
 * @internal
 */
export enum Action {
    Compress = 1,
    Decompress = 2,
    Transfer = 3,
}

/**
 * @internal
 */
const shuffleArray = <T>(array: T[]): T[] => {
    for (let i = array.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [array[i], array[j]] = [array[j], array[i]];
    }
    return array;
};

/**
 * Select a random token pool info from the token pool infos.
 *
 * @param infos The token pool infos
 *
 * @returns A random token pool info
 */
export function selectTokenPoolInfo(infos: TokenPoolInfo[]): TokenPoolInfo {
    infos = shuffleArray(infos);

    // filter only infos that are initialized
    infos = infos.filter(info => info.isInitialized);

    // Return a single random token pool info
    return infos[0];
}

/**
 * Select one or multiple token pool infos from the token pool infos.
 *
 * Use this function for `decompress`.
 *
 * For `compress`, `mintTo` use {@link selectTokenPoolInfo} instead.
 *
 * @param infos             The token pool infos
 * @param decompressAmount  The amount of tokens to withdraw
 *
 * @returns Array with one or more token pool infos.
 */
export function selectTokenPoolInfosForDecompression(
    infos: TokenPoolInfo[],
    decompressAmount: number | BN,
): TokenPoolInfo[] {
    infos = shuffleArray(infos);
    // Find the first info where balance is 10x the requested amount
    const sufficientBalanceInfo = infos.find(info =>
        info.balance.gte(new BN(decompressAmount).mul(new BN(10))),
    );
    // filter only infos that are initialized
    infos = infos.filter(info => info.isInitialized);
    // If none found, return all infos
    return sufficientBalanceInfo ? [sufficientBalanceInfo] : infos;
}
