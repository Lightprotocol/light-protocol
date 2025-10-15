import { AccountInfo, Commitment, PublicKey } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    unpackAccount as unpackAccountSPL,
    TokenAccountNotFoundError,
    TokenInvalidAccountOwnerError,
} from '@solana/spl-token';
import {
    Rpc,
    CTOKEN_PROGRAM_ID,
    MerkleContext,
    CompressedAccountWithMerkleContext,
} from '@lightprotocol/stateless.js';
import { Buffer } from 'buffer';
import BN from 'bn.js';

export interface Account {
    address: PublicKey;
    mint: PublicKey;
    owner: PublicKey;
    amount: bigint;
    delegate: PublicKey | null;
    delegatedAmount: bigint;
    isInitialized: boolean;
    isFrozen: boolean;
    isNative: boolean;
    rentExemptReserve: bigint | null;
    closeAuthority: PublicKey | null;
    tlvData: Buffer;
}

export enum AccountState {
    Uninitialized = 0,
    Initialized = 1,
    Frozen = 2,
}

export interface ParsedTokenAccount {
    compressedAccount: CompressedAccountWithMerkleContext;
    parsed: Account;
}

function parseTokenData(data: Buffer): {
    mint: PublicKey;
    owner: PublicKey;
    amount: BN;
    delegate: PublicKey | null;
    state: number;
    tlv: Buffer | null;
} | null {
    if (!data || data.length === 0) return null;

    try {
        let offset = 0;
        const mint = new PublicKey(data.slice(offset, offset + 32));
        offset += 32;
        const owner = new PublicKey(data.slice(offset, offset + 32));
        offset += 32;
        const amount = new BN(data.slice(offset, offset + 8), 'le');
        offset += 8;
        const delegateOption = data[offset];
        offset += 1;
        const delegate = delegateOption
            ? new PublicKey(data.slice(offset, offset + 32))
            : null;
        offset += 32;
        const state = data[offset];
        offset += 1;
        const tlvOption = data[offset];
        offset += 1;
        const tlv = tlvOption ? data.slice(offset) : null;

        return {
            mint,
            owner,
            amount,
            delegate,
            state,
            tlv,
        };
    } catch (error) {
        console.error('Token data parsing error:', error);
        return null;
    }
}

function convertTokenDataToAccount(
    address: PublicKey,
    tokenData: {
        mint: PublicKey;
        owner: PublicKey;
        amount: BN;
        delegate: PublicKey | null;
        state: number;
        tlv: Buffer | null;
    },
): Account {
    return {
        address,
        mint: tokenData.mint,
        owner: tokenData.owner,
        amount: BigInt(tokenData.amount.toString()),
        delegate: tokenData.delegate,
        delegatedAmount: BigInt(0),
        isInitialized: tokenData.state !== AccountState.Uninitialized,
        isFrozen: tokenData.state === AccountState.Frozen,
        isNative: false,
        rentExemptReserve: null,
        closeAuthority: null,
        tlvData: tokenData.tlv ? Buffer.from(tokenData.tlv) : Buffer.alloc(0),
    };
}

/** normalize compressed account to account info */
export function toAccountInfo(
    compressedAccount: CompressedAccountWithMerkleContext,
): AccountInfo<Buffer> {
    // we must define Buffer type explicitly.
    const dataDiscriminatorBuffer: Buffer = Buffer.from(
        compressedAccount.data!.discriminator,
    );
    const dataBuffer: Buffer = Buffer.from(compressedAccount.data!.data);
    const data: Buffer = Buffer.concat([dataDiscriminatorBuffer, dataBuffer]);

    return {
        executable: false,
        owner: compressedAccount.owner,
        lamports: compressedAccount.lamports.toNumber(),
        data,
        rentEpoch: undefined,
    };
}

export function parseCTokenOnchain(
    address: PublicKey,
    accountInfo: AccountInfo<Buffer>,
): {
    accountInfo: AccountInfo<Buffer>;
    merkleContext: undefined;
    parsed: Account;
    isCompressed: false;
} {
    const parsed = parseTokenData(accountInfo.data);
    if (!parsed) throw new Error('Invalid token data');
    return {
        accountInfo,
        merkleContext: undefined,
        parsed: convertTokenDataToAccount(address, parsed),
        isCompressed: false,
    };
}

export function parseCTokenCompressed(
    address: PublicKey,
    compressedAccount: CompressedAccountWithMerkleContext,
): {
    accountInfo: AccountInfo<Buffer>;
    merkleContext: MerkleContext;
    parsed: Account;
    isCompressed: true;
} {
    const parsed = parseTokenData(compressedAccount.data!.data);
    if (!parsed) throw new Error('Invalid token data');
    return {
        accountInfo: toAccountInfo(compressedAccount),
        merkleContext: {
            treeInfo: compressedAccount.treeInfo,
            hash: compressedAccount.hash,
            leafIndex: compressedAccount.leafIndex,
            proveByIndex: compressedAccount.proveByIndex,
        },
        parsed: convertTokenDataToAccount(address, parsed),
        isCompressed: true,
    };
}

/**
 * Retrieve information about a token account (SPL or compressed)
 *
 * @param rpc        RPC connection to use
 * @param address    Token account address
 * @param commitment Desired level of commitment for querying the state
 * @param programId  Token program ID. If not provided, tries all programs concurrently to auto-detect
 *
 * @return Token account information with compression context if applicable
 */
export async function getAccountInterface(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    parsed: Account;
    isCompressed: boolean;
    merkleContext?: MerkleContext;
}> {
    // Auto-detect: try all programs in parallel (4 calls max)
    if (!programId) {
        const [
            tokenResult,
            token2022Result,
            ctokenOnchainResult,
            ctokenCompressedResult,
        ] = await Promise.allSettled([
            // 1. TOKEN_PROGRAM_ID onchain
            rpc.getAccountInfo(address, commitment).then(info => {
                if (!info || !info.owner.equals(TOKEN_PROGRAM_ID)) {
                    throw new Error('Not a TOKEN_PROGRAM_ID account');
                }
                const account = unpackAccountSPL(
                    address,
                    info,
                    TOKEN_PROGRAM_ID,
                );
                return {
                    accountInfo: info,
                    parsed: account,
                    isCompressed: false,
                    merkleContext: undefined,
                };
            }),
            // 2. TOKEN_2022_PROGRAM_ID onchain
            rpc.getAccountInfo(address, commitment).then(info => {
                if (!info || !info.owner.equals(TOKEN_2022_PROGRAM_ID)) {
                    throw new Error('Not a TOKEN_2022_PROGRAM_ID account');
                }
                const account = unpackAccountSPL(
                    address,
                    info,
                    TOKEN_2022_PROGRAM_ID,
                );
                return {
                    accountInfo: info,
                    parsed: account,
                    isCompressed: false,
                    merkleContext: undefined,
                };
            }),
            // 3. CTOKEN_PROGRAM_ID onchain
            rpc.getAccountInfo(address, commitment).then(info => {
                if (!info || !info.owner.equals(CTOKEN_PROGRAM_ID)) {
                    throw new Error('Not a CTOKEN onchain account');
                }
                return parseCTokenOnchain(address, info);
            }),
            // 4. CTOKEN_PROGRAM_ID compressed
            rpc.getCompressedTokenAccountsByOwner(address).then(result => {
                const compressedAccount =
                    result.items.length > 0
                        ? result.items[0].compressedAccount
                        : null;
                if (!compressedAccount?.data?.data.length) {
                    throw new Error('Not a compressed token account');
                }
                if (!compressedAccount.owner.equals(CTOKEN_PROGRAM_ID)) {
                    throw new Error('Invalid owner for compressed token');
                }
                return parseCTokenCompressed(address, compressedAccount);
            }),
        ]);

        // Return whichever succeeded
        if (tokenResult.status === 'fulfilled') {
            return tokenResult.value;
        }
        if (token2022Result.status === 'fulfilled') {
            return token2022Result.value;
        }
        if (ctokenOnchainResult.status === 'fulfilled') {
            return ctokenOnchainResult.value;
        }
        if (ctokenCompressedResult.status === 'fulfilled') {
            return ctokenCompressedResult.value;
        }

        // None succeeded - account not found
        throw new Error(
            `Token account not found: ${address.toString()}. ` +
                `Tried TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, and CTOKEN_PROGRAM_ID (both onchain and compressed).`,
        );
    }

    // Handle specific programId
    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        const [onchainResult, compressedResult] = await Promise.allSettled([
            rpc.getAccountInfo(address, commitment),
            rpc.getCompressedTokenAccountsByOwner(address),
        ]);

        const onchainAccount =
            onchainResult.status === 'fulfilled' ? onchainResult.value : null;
        const compressedAccount =
            compressedResult.status === 'fulfilled' &&
            compressedResult.value.items.length > 0
                ? compressedResult.value.items[0].compressedAccount
                : null;

        if (onchainAccount) {
            if (!onchainAccount.owner.equals(programId)) {
                throw new Error(
                    `Invalid owner ${onchainAccount.owner.toBase58()} for token layout`,
                );
            }

            if (compressedAccount) {
                throw Error('Expected no compressed token account');
            }

            return parseCTokenOnchain(address, onchainAccount);
        }

        if (
            compressedAccount &&
            compressedAccount.data &&
            compressedAccount.data.data.length > 0
        ) {
            if (!compressedAccount.owner.equals(programId)) {
                throw new Error(
                    `Invalid owner ${compressedAccount.owner.toBase58()} for token layout`,
                );
            }

            return parseCTokenCompressed(address, compressedAccount);
        }

        throw new TokenAccountNotFoundError();
    }

    if (
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        const info = await rpc.getAccountInfo(address, commitment);
        if (!info) {
            throw new TokenAccountNotFoundError();
        }

        const account = unpackAccountSPL(address, info, programId);

        return {
            accountInfo: info,
            parsed: account,
            isCompressed: false,
            merkleContext: undefined,
        };
    }

    throw new Error(`Unsupported program ID: ${programId.toBase58()}`);
}
