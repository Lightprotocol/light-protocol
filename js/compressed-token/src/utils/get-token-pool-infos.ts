import { Commitment, PublicKey } from '@solana/web3.js';
import { unpackAccount } from '@solana/spl-token';
import { CompressedTokenProgram } from '../program';
import { bn, Rpc } from '@lightprotocol/stateless.js';
import BN from 'bn.js';

/**
 * SPL interface PDA info.
 */
export type SplInterfaceInfo = {
    /**
     * The mint of the SPL interface
     */
    mint: PublicKey;
    /**
     * The SPL interface address
     */
    splInterfacePda: PublicKey;
    /**
     * The token program of the SPL interface
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
     * Whether the SPL interface is initialized
     */
    isInitialized: boolean;
    /**
     * The balance of the SPL interface
     */
    balance: BN;
    /**
     * The index of the SPL interface
     */
    poolIndex: number;
    /**
     * The bump used to derive the SPL interface PDA
     */
    bump: number;
};

/**
 * @deprecated Use {@link SplInterfaceInfo} instead.
 * This type maintains backward compatibility by including both tokenPoolPda and splInterfacePda.
 * Both properties point to the same PublicKey value.
 */
export type TokenPoolInfo = {
    /**
     * The mint of the SPL interface
     */
    mint: PublicKey;
    /**
     * @deprecated Use splInterfacePda instead.
     */
    tokenPoolPda: PublicKey;
    /**
     * The SPL interface address (new name).
     * For backward compatibility, tokenPoolPda is also available.
     */
    splInterfacePda: PublicKey;
    /**
     * The token program of the SPL interface
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
     * Whether the SPL interface is initialized
     */
    isInitialized: boolean;
    /**
     * The balance of the SPL interface
     */
    balance: BN;
    /**
     * The index of the SPL interface
     */
    poolIndex: number;
    /**
     * The bump used to derive the SPL interface PDA
     */
    bump: number;
};

/**
 * Convert SplInterfaceInfo to TokenPoolInfo for backward compatibility.
 * @internal
 */
export function toTokenPoolInfo(info: SplInterfaceInfo): TokenPoolInfo {
    return {
        mint: info.mint,
        tokenPoolPda: info.splInterfacePda,
        splInterfacePda: info.splInterfacePda,
        tokenProgram: info.tokenProgram,
        activity: info.activity,
        isInitialized: info.isInitialized,
        balance: info.balance,
        poolIndex: info.poolIndex,
        bump: info.bump,
    };
}

/**
 * Convert TokenPoolInfo to SplInterfaceInfo.
 * @internal
 */
function toSplInterfaceInfo(info: TokenPoolInfo): SplInterfaceInfo {
    return {
        mint: info.mint,
        splInterfacePda: info.tokenPoolPda ?? info.splInterfacePda,
        tokenProgram: info.tokenProgram,
        activity: info.activity,
        isInitialized: info.isInitialized,
        balance: info.balance,
        poolIndex: info.poolIndex,
        bump: info.bump,
    };
}

/**
 * Derive SplInterfaceInfo for an SPL interface that will be initialized in the
 * same transaction. Use this when you need to create an SPL interface and
 * compress in a single transaction.
 *
 * @param mint           The mint of the SPL interface
 * @param tokenProgramId The token program (TOKEN_PROGRAM_ID or TOKEN_2022_PROGRAM_ID)
 * @param poolIndex      The pool index. Default 0.
 *
 * @returns SplInterfaceInfo for the to-be-initialized interface
 */
export function deriveSplInterfaceInfo(
    mint: PublicKey,
    tokenProgramId: PublicKey,
    poolIndex = 0,
): SplInterfaceInfo {
    const [splInterfacePda, bump] =
        CompressedTokenProgram.deriveSplInterfacePdaWithIndex(mint, poolIndex);

    return {
        mint,
        splInterfacePda,
        tokenProgram: tokenProgramId,
        activity: undefined,
        balance: bn(0),
        isInitialized: true,
        poolIndex,
        bump,
    };
}

/**
 * Check if the SPL interface info is initialized and has a balance.
 * @param mint The mint of the SPL interface
 * @param splInterfaceInfo The SPL interface info (or TokenPoolInfo for backward compatibility)
 * @returns True if the SPL interface info is initialized and has a balance
 */
export function checkSplInterfaceInfo(
    splInterfaceInfo: SplInterfaceInfo | TokenPoolInfo,
    mint: PublicKey,
): boolean {
    // Handle backward compatibility with TokenPoolInfo
    // TokenPoolInfo has both tokenPoolPda and splInterfacePda, so we can use either
    const info: SplInterfaceInfo =
        'tokenPoolPda' in splInterfaceInfo
            ? toSplInterfaceInfo(splInterfaceInfo as TokenPoolInfo)
            : (splInterfaceInfo as SplInterfaceInfo);

    if (!info.mint.equals(mint)) {
        throw new Error(`SplInterface mint does not match the provided mint.`);
    }

    if (!info.isInitialized) {
        throw new Error(
            `SplInterface is not initialized. Please create an SPL interface for mint: ${mint.toBase58()} via createSplInterface().`,
        );
    }
    return true;
}

/**
 * Get the SPL interface infos for a given mint.
 * @param rpc         The RPC client
 * @param mint        The mint of the SPL interface
 * @param commitment  The commitment to use
 *
 * @returns The SPL interface infos
 */
export async function getSplInterfaceInfos(
    rpc: Rpc,
    mint: PublicKey,
    commitment?: Commitment,
): Promise<SplInterfaceInfo[]> {
    const addressesAndBumps = Array.from({ length: 5 }, (_, i) =>
        CompressedTokenProgram.deriveSplInterfacePdaWithIndex(mint, i),
    );

    const accountInfos = await rpc.getMultipleAccountsInfo(
        addressesAndBumps.map(addressAndBump => addressAndBump[0]),
        commitment,
    );

    if (accountInfos[0] === null) {
        throw new Error(
            `SplInterface not found. Please create an SPL interface for mint: ${mint.toBase58()} via createSplInterface().`,
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
                splInterfacePda: addressesAndBumps[i][0],
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
            splInterfacePda: parsedInfo.address,
            tokenProgram,
            activity: undefined,
            balance: bn(parsedInfo.amount.toString()),
            isInitialized: true,
            poolIndex: i,
            bump: addressesAndBumps[i][1],
        };
    });
}

export type SplInterfaceActivity = {
    signature: string;
    amount: BN;
    action: Action;
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
 * Select a random SPL interface info from the SPL interface infos.
 *
 * For `decompress`, use {@link selectSplInterfaceInfosForDecompression} instead.
 *
 * @param infos The SPL interface infos
 *
 * @returns A random SPL interface info
 */
export function selectSplInterfaceInfo(
    infos: SplInterfaceInfo[],
): SplInterfaceInfo {
    const shuffledInfos = shuffleArray(infos);

    // filter only infos that are initialized
    const filteredInfos = shuffledInfos.filter(info => info.isInitialized);

    if (filteredInfos.length === 0) {
        throw new Error(
            'Please pass at least one initialized SPL interface info.',
        );
    }

    // Return a single random SPL interface info
    return filteredInfos[0];
}

/**
 * Select one or multiple SPL interface infos from the SPL interface infos.
 *
 * Use this function for `decompress`.
 *
 * For `compress`, `mintTo` use {@link selectSplInterfaceInfo} instead.
 *
 * @param infos             The SPL interface infos
 * @param decompressAmount  The amount of tokens to withdraw
 *
 * @returns Array with one or more SPL interface infos.
 */
export function selectSplInterfaceInfosForDecompression(
    infos: SplInterfaceInfo[],
    decompressAmount: number | BN,
): SplInterfaceInfo[] {
    if (infos.length === 0) {
        throw new Error('Please pass at least one SPL interface info.');
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
            'All provided SPL interface balances are zero. Please pass recent SPL interface infos.',
        );
    }

    // If none found, return all infos
    return sufficientBalanceInfo ? [sufficientBalanceInfo] : infos;
}

// =============================================================================
// DEPRECATED TYPES AND FUNCTIONS - Use the new SplInterface* names instead
// =============================================================================

/**
 * @deprecated Use {@link SplInterfaceActivity} instead.
 */
export type TokenPoolActivity = SplInterfaceActivity;

/**
 * @deprecated Use {@link deriveSplInterfaceInfo} instead.
 */
export function deriveTokenPoolInfo(
    mint: PublicKey,
    tokenProgramId: PublicKey,
    poolIndex = 0,
): TokenPoolInfo {
    const info = deriveSplInterfaceInfo(mint, tokenProgramId, poolIndex);
    return toTokenPoolInfo(info);
}

/**
 * @deprecated Use {@link checkSplInterfaceInfo} instead.
 */
export function checkTokenPoolInfo(
    tokenPoolInfo: TokenPoolInfo,
    mint: PublicKey,
): boolean {
    return checkSplInterfaceInfo(toSplInterfaceInfo(tokenPoolInfo), mint);
}

/**
 * @deprecated Use {@link getSplInterfaceInfos} instead.
 */
export async function getTokenPoolInfos(
    rpc: Rpc,
    mint: PublicKey,
    commitment?: Commitment,
): Promise<TokenPoolInfo[]> {
    const infos = await getSplInterfaceInfos(rpc, mint, commitment);
    return infos.map(toTokenPoolInfo);
}

/**
 * @deprecated Use {@link selectSplInterfaceInfo} instead.
 */
export function selectTokenPoolInfo(infos: TokenPoolInfo[]): TokenPoolInfo {
    const splInfos = infos.map(toSplInterfaceInfo);
    const selected = selectSplInterfaceInfo(splInfos);
    return toTokenPoolInfo(selected);
}

/**
 * @deprecated Use {@link selectSplInterfaceInfosForDecompression} instead.
 */
export function selectTokenPoolInfosForDecompression(
    infos: TokenPoolInfo[],
    decompressAmount: number | BN,
): TokenPoolInfo[] {
    const splInfos = infos.map(toSplInterfaceInfo);
    const selected = selectSplInterfaceInfosForDecompression(
        splInfos,
        decompressAmount,
    );
    return selected.map(toTokenPoolInfo);
}
