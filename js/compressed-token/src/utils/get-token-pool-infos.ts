import { Commitment, PublicKey } from '@solana/web3.js';
import { unpackAccount } from '@solana/spl-token';
import { CompressedTokenProgram } from '../program';
import { bn, Rpc } from '@lightprotocol/stateless.js';
import BN from 'bn.js';

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
    const addressesAndBumps = Array.from({ length: 5 }, (_, i) =>
        CompressedTokenProgram.deriveTokenPoolPdaWithIndex(mint, i),
    );

    const accountInfos = await rpc.getMultipleAccountsInfo(
        addressesAndBumps.map(addressAndBump => addressAndBump[0]),
        commitment,
    );

    if (accountInfos[0] === null) {
        throw new Error(
            `TokenPool not found. Please create a compressed token pool for mint: ${mint.toBase58()} via createTokenPool().`,
        );
    }

    const parsedInfos = addressesAndBumps.map((addressAndBump, i) =>
        accountInfos[i]
            ? unpackAccount(
                  addressAndBump[0],
                  accountInfos[i],
                  accountInfos[i].owner,
              )
            : null,
    );

    const tokenProgram = accountInfos[0].owner;
    return parsedInfos.map((parsedInfo, i) => {
        if (!parsedInfo) {
            return {
                mint,
                tokenPoolPda: addressesAndBumps[i][0],
                tokenProgram,
                activity: undefined,
                balance: bn(0),
                isInitialized: false,
                poolIndex: i,
                bump: addressesAndBumps[i][1],
            };
        }

        return {
            mint,
            tokenPoolPda: parsedInfo.address,
            tokenProgram,
            activity: undefined,
            balance: bn(parsedInfo.amount.toString()),
            isInitialized: true,
            poolIndex: i,
            bump: addressesAndBumps[i][1],
        };
    });
}

export type TokenPoolActivity = {
    signature: string;
    amount: BN;
    action: Action;
};

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
    /**
     * The index of the token pool
     */
    poolIndex: number;
    /**
     * The bump used to derive the token pool pda
     */
    bump: number;
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
 * For `compress` and `mintTo` instructions only.
 * Select a random token pool info from the token pool infos.
 *
 * For `decompress`, use {@link selectTokenPoolInfosForDecompression} instead.
 *
 * @param infos The token pool infos
 *
 * @returns A random token pool info
 */
export function selectTokenPoolInfo(infos: TokenPoolInfo[]): TokenPoolInfo {
    const shuffledInfos = shuffleArray(infos);

    // filter only infos that are initialized
    const filteredInfos = shuffledInfos.filter(info => info.isInitialized);

    if (filteredInfos.length === 0) {
        throw new Error(
            'Please pass at least one initialized token pool info.',
        );
    }

    // Return a single random token pool info
    return filteredInfos[0];
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
    if (infos.length === 0) {
        throw new Error('Please pass at least one token pool info.');
    }

    infos = shuffleArray(infos);
    // Find the first info where balance is 10x the requested amount
    const sufficientBalanceInfo = infos.find(info =>
        info.balance.gte(bn(decompressAmount).mul(bn(10))),
    );

    // filter only infos that are initialized
    infos = infos
        .filter(info => info.isInitialized)
        .sort((a, b) => a.poolIndex - b.poolIndex);

    const allBalancesZero = infos.every(info => info.balance.isZero());
    if (allBalancesZero) {
        throw new Error(
            'All provided token pool balances are zero. Please pass recent token pool infos.',
        );
    }

    // If none found, return all infos
    return sufficientBalanceInfo ? [sufficientBalanceInfo] : infos;
}
