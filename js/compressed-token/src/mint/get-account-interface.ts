import { AccountInfo, Commitment, PublicKey } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    unpackAccount as unpackAccountSPL,
    TokenAccountNotFoundError,
    getAssociatedTokenAddressSync,
    AccountState,
    AccountLayout,
    Account,
} from '@solana/spl-token';
import {
    Rpc,
    CTOKEN_PROGRAM_ID,
    MerkleContext,
    CompressedAccountWithMerkleContext,
    ParsedTokenAccount,
} from '@lightprotocol/stateless.js';
import { Buffer } from 'buffer';
import BN from 'bn.js';
import { getAtaProgramId } from '../utils';

// Re-export types that are used in the interface
export { Account, AccountState } from '@solana/spl-token';
export { ParsedTokenAccount } from '@lightprotocol/stateless.js';

export interface TokenAccountSource {
    type:
        | 'spl-onchain'
        | 'token2022-onchain'
        | 'ctoken-onchain'
        | 'ctoken-compressed';
    address: PublicKey;
    amount: bigint;
    accountInfo: AccountInfo<Buffer>;
    loadContext?: MerkleContext;
    parsed: Account;
}

export interface AccountInterface {
    accountInfo: AccountInfo<Buffer>;
    parsed: Account;
    isCold: boolean;
    loadContext?: MerkleContext;
    _sources?: TokenAccountSource[];
    _needsConsolidation?: boolean;
    _hasDelegate?: boolean;
    _anyFrozen?: boolean;
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

export function convertTokenDataToAccount(
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
    loadContext: undefined;
    parsed: Account;
    isCold: false;
} {
    const parsed = parseTokenData(accountInfo.data);
    if (!parsed) throw new Error('Invalid token data');
    return {
        accountInfo,
        loadContext: undefined,
        parsed: convertTokenDataToAccount(address, parsed),
        isCold: false,
    };
}

export function parseCTokenCompressed(
    address: PublicKey,
    compressedAccount: CompressedAccountWithMerkleContext,
): {
    accountInfo: AccountInfo<Buffer>;
    loadContext: MerkleContext;
    parsed: Account;
    isCold: true;
} {
    const parsed = parseTokenData(compressedAccount.data!.data);
    if (!parsed) throw new Error('Invalid token data');
    return {
        accountInfo: toAccountInfo(compressedAccount),
        loadContext: {
            treeInfo: compressedAccount.treeInfo,
            hash: compressedAccount.hash,
            leafIndex: compressedAccount.leafIndex,
            proveByIndex: compressedAccount.proveByIndex,
        },
        parsed: convertTokenDataToAccount(address, parsed),
        isCold: true,
    };
}
/**
 * Retrieve information about a token account (SPL, T22, C-Token)
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
): Promise<AccountInterface> {
    return _getAccountInterface(rpc, address, commitment, programId, undefined);
}

/** Retrieve associated token account for a given owner and mint. */
export async function getAtaInterface(
    rpc: Rpc,
    owner: PublicKey,
    mint: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
): Promise<AccountInterface> {
    return _getAccountInterface(rpc, undefined, commitment, programId, {
        owner,
        mint,
    });
}

/**
 * Helper: Try to fetch SPL Token onchain account
 */
async function _tryFetchSplOnchain(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    parsed: Account;
    isCold: false;
    loadContext: undefined;
}> {
    const info = await rpc.getAccountInfo(address, commitment);
    if (!info || !info.owner.equals(TOKEN_PROGRAM_ID)) {
        throw new Error('Not a TOKEN_PROGRAM_ID account');
    }
    const account = unpackAccountSPL(address, info, TOKEN_PROGRAM_ID);
    return {
        accountInfo: info,
        parsed: account,
        isCold: false,
        loadContext: undefined,
    };
}

/**
 * Helper: Try to fetch Token-2022 onchain account
 */
async function _tryFetchToken2022Onchain(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    parsed: Account;
    isCold: false;
    loadContext: undefined;
}> {
    const info = await rpc.getAccountInfo(address, commitment);
    if (!info || !info.owner.equals(TOKEN_2022_PROGRAM_ID)) {
        throw new Error('Not a TOKEN_2022_PROGRAM_ID account');
    }
    const account = unpackAccountSPL(address, info, TOKEN_2022_PROGRAM_ID);
    return {
        accountInfo: info,
        parsed: account,
        isCold: false,
        loadContext: undefined,
    };
}

/**
 * Helper: Try to fetch CToken onchain account
 */
async function _tryFetchCTokenOnchain(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    loadContext: undefined;
    parsed: Account;
    isCold: false;
}> {
    const info = await rpc.getAccountInfo(address, commitment);
    if (!info || !info.owner.equals(CTOKEN_PROGRAM_ID)) {
        throw new Error('Not a CTOKEN onchain account');
    }
    return parseCTokenOnchain(address, info);
}

/**
 * Helper: Try to fetch compressed token account by owner+mint
 */
async function _tryFetchCompressedByOwner(
    rpc: Rpc,
    owner: PublicKey,
    mint: PublicKey,
    ataAddress: PublicKey,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    loadContext: MerkleContext;
    parsed: Account;
    isCold: true;
}> {
    const result = await rpc.getCompressedTokenAccountsByOwner(owner, {
        mint,
    });
    const compressedAccount =
        result.items.length > 0 ? result.items[0].compressedAccount : null;
    if (!compressedAccount?.data?.data.length) {
        throw new Error('Not a compressed token account');
    }
    if (!compressedAccount.owner.equals(CTOKEN_PROGRAM_ID)) {
        throw new Error('Invalid owner for compressed token');
    }
    return parseCTokenCompressed(ataAddress, compressedAccount);
}

/**
 * Helper: Try to fetch compressed token account by address (for non-ATA ctokens)
 */
async function _tryFetchCompressedByAddress(
    rpc: Rpc,
    address: PublicKey,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    loadContext: MerkleContext;
    parsed: Account;
    isCold: true;
}> {
    const result = await rpc.getCompressedTokenAccountsByOwner(address);
    const compressedAccount =
        result.items.length > 0 ? result.items[0].compressedAccount : null;
    if (!compressedAccount?.data?.data.length) {
        throw new Error('Not a compressed token account');
    }
    if (!compressedAccount.owner.equals(CTOKEN_PROGRAM_ID)) {
        throw new Error('Invalid owner for compressed token');
    }
    return parseCTokenCompressed(address, compressedAccount);
}

// TODO: add test
//
// TODO: implement actual solution for compressed token accounts for vaults for
// spl/t22 mints.
/**
 * @internal
 * Retrieve information about a token account (SPL, T22, C-Token)
 *
 * @param rpc        RPC connection to use
 * @param address    Token account address
 * @param commitment Desired level of commitment for querying the state
 * @param programId  Token program ID. If not provided, tries all programs concurrently to auto-detect
 * @param fetchByOwner ATA options. If provided, tries to fetch the compressible side by owner and mint instead of address
 *
 * @return Token account information with compression context if applicable
 */
async function _getAccountInterface(
    rpc: Rpc,
    address?: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
    fetchByOwner?: {
        owner: PublicKey;
        mint: PublicKey;
    },
): Promise<AccountInterface> {
    if (!address && !fetchByOwner) {
        throw new Error('One of Address or fetchByOwner is required');
    }
    if (address && fetchByOwner) {
        throw new Error('Only one of Address or fetchByOwner can be provided');
    }

    // Auto-detect: try all programs in parallel
    if (!programId) {
        // Derive ATA addresses for each program (or use provided address)
        const cTokenAta = address
            ? address
            : getAssociatedTokenAddressSync(
                  fetchByOwner!.mint,
                  fetchByOwner!.owner,
                  false,
                  CTOKEN_PROGRAM_ID,
                  getAtaProgramId(CTOKEN_PROGRAM_ID),
              );
        const splTokenAta = address
            ? address
            : getAssociatedTokenAddressSync(
                  fetchByOwner!.mint,
                  fetchByOwner!.owner,
                  false,
                  TOKEN_PROGRAM_ID,
                  getAtaProgramId(TOKEN_PROGRAM_ID),
              );
        const token2022Ata = address
            ? address
            : getAssociatedTokenAddressSync(
                  fetchByOwner!.mint,
                  fetchByOwner!.owner,
                  false,
                  TOKEN_2022_PROGRAM_ID,
                  getAtaProgramId(TOKEN_2022_PROGRAM_ID),
              );

        const results = await Promise.allSettled([
            // 1. SPL Token onchain
            _tryFetchSplOnchain(rpc, splTokenAta, commitment),
            // 2. Token-2022 onchain
            _tryFetchToken2022Onchain(rpc, token2022Ata, commitment),
            // 3. CToken onchain
            _tryFetchCTokenOnchain(rpc, cTokenAta, commitment),
            // 4. CToken compressed (all compressed tokens are owned by CTOKEN_PROGRAM_ID)
            fetchByOwner
                ? _tryFetchCompressedByOwner(
                      rpc,
                      fetchByOwner.owner,
                      fetchByOwner.mint,
                      cTokenAta,
                  )
                : _tryFetchCompressedByAddress(rpc, address!),
        ]);

        // Collect all successful results
        const sources: TokenAccountSource[] = [];
        const successfulResults: Array<{
            accountInfo: AccountInfo<Buffer>;
            parsed: Account;
            isCold: boolean;
            loadContext?: MerkleContext;
        }> = [];

        for (let i = 0; i < results.length; i++) {
            const result = results[i];
            if (result.status === 'fulfilled') {
                const value = result.value;
                successfulResults.push(value);

                let type: TokenAccountSource['type'];
                let addr: PublicKey;

                if (i === 0) {
                    type = 'spl-onchain';
                    addr = splTokenAta;
                } else if (i === 1) {
                    type = 'token2022-onchain';
                    addr = token2022Ata;
                } else if (i === 2) {
                    type = 'ctoken-onchain';
                    addr = cTokenAta;
                } else {
                    type = 'ctoken-compressed';
                    addr = cTokenAta;
                }

                sources.push({
                    type,
                    address: addr,
                    amount: value.parsed.amount,
                    accountInfo: value.accountInfo,
                    loadContext: value.loadContext,
                    parsed: value.parsed,
                });
            }
        }

        // None succeeded - account not found
        if (sources.length === 0) {
            throw new Error(
                `Token account not found. ` +
                    `Tried TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, and CTOKEN_PROGRAM_ID (both onchain and compressed).`,
            );
        }

        // Priority order: CToken onchain > CToken compressed > SPL/T22
        const priority: TokenAccountSource['type'][] = [
            'ctoken-onchain',
            'ctoken-compressed',
            'spl-onchain',
            'token2022-onchain',
        ];

        sources.sort((a, b) => {
            const aIdx = priority.indexOf(a.type);
            const bIdx = priority.indexOf(b.type);
            return aIdx - bIdx;
        });

        // Aggregate balance from all sources
        const totalAmount = sources.reduce(
            (sum, src) => sum + src.amount,
            BigInt(0),
        );

        // Use the highest priority source as base
        const primarySource = sources[0];

        // Check for concerns
        const hasDelegate = sources.some(src => src.parsed.delegate !== null);
        const anyFrozen = sources.some(src => src.parsed.isFrozen);
        const needsConsolidation = sources.length > 1;

        // Create unified account with aggregated balance
        const unifiedAccount: Account = {
            ...primarySource.parsed,
            address: cTokenAta,
            amount: totalAmount,
        };

        const isCold = primarySource.type === 'ctoken-compressed';

        return {
            accountInfo: primarySource.accountInfo!,
            parsed: unifiedAccount,
            isCold,
            loadContext: primarySource.loadContext,
            _sources: sources,
            _needsConsolidation: needsConsolidation,
            _hasDelegate: hasDelegate,
            _anyFrozen: anyFrozen,
        };
    }

    // Handle specific programId - CTOKEN
    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        // Derive address if not provided
        if (!address) {
            if (!fetchByOwner) {
                throw new Error('fetchByOwner is required');
            }
            address = getAssociatedTokenAddressSync(
                fetchByOwner.mint,
                fetchByOwner.owner,
                false,
                CTOKEN_PROGRAM_ID,
                getAtaProgramId(CTOKEN_PROGRAM_ID),
            );
        }

        const [onchainResult, compressedResult] = await Promise.allSettled([
            rpc.getAccountInfo(address, commitment),
            // Fetch compressed: by owner+mint for ATAs, by address for non-ATAs
            fetchByOwner
                ? rpc.getCompressedTokenAccountsByOwner(fetchByOwner.owner, {
                      mint: fetchByOwner.mint,
                  })
                : rpc.getCompressedTokenAccountsByOwner(address),
        ]);

        const onchainAccount =
            onchainResult.status === 'fulfilled' ? onchainResult.value : null;
        const compressedAccounts =
            compressedResult.status === 'fulfilled'
                ? compressedResult.value.items.map(
                      item => item.compressedAccount,
                  )
                : [];

        const sources: TokenAccountSource[] = [];

        // Collect onchain CToken account
        if (onchainAccount && onchainAccount.owner.equals(programId)) {
            const parsed = parseCTokenOnchain(address, onchainAccount);
            sources.push({
                type: 'ctoken-onchain',
                address,
                amount: parsed.parsed.amount,
                accountInfo: onchainAccount,
                parsed: parsed.parsed,
            });
        }

        // Collect compressed CToken accounts
        for (const compressedAccount of compressedAccounts) {
            if (
                compressedAccount &&
                compressedAccount.data &&
                compressedAccount.data.data.length > 0 &&
                compressedAccount.owner.equals(programId)
            ) {
                const parsed = parseCTokenCompressed(
                    address,
                    compressedAccount,
                );
                sources.push({
                    type: 'ctoken-compressed',
                    address,
                    amount: parsed.parsed.amount,
                    accountInfo: parsed.accountInfo,
                    loadContext: parsed.loadContext,
                    parsed: parsed.parsed,
                });
            }
        }

        if (sources.length === 0) {
            throw new TokenAccountNotFoundError();
        }

        // Priority: onchain > compressed
        sources.sort((a, b) => {
            if (a.type === 'ctoken-onchain' && b.type === 'ctoken-compressed')
                return -1;
            if (a.type === 'ctoken-compressed' && b.type === 'ctoken-onchain')
                return 1;
            return 0;
        });

        // Aggregate balance
        const totalAmount = sources.reduce(
            (sum, src) => sum + src.amount,
            BigInt(0),
        );

        const primarySource = sources[0];
        const hasDelegate = sources.some(src => src.parsed.delegate !== null);
        const anyFrozen = sources.some(src => src.parsed.isFrozen);
        const needsConsolidation = sources.length > 1;

        const unifiedAccount: Account = {
            ...primarySource.parsed,
            address,
            amount: totalAmount,
        };

        return {
            accountInfo: primarySource.accountInfo!,
            parsed: unifiedAccount,
            isCold: primarySource.type === 'ctoken-compressed',
            loadContext: primarySource.loadContext,
            _sources: sources,
            _needsConsolidation: needsConsolidation,
            _hasDelegate: hasDelegate,
            _anyFrozen: anyFrozen,
        };
    }

    // Handle specific programId - SPL Token or Token-2022
    if (
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        // Derive address if not provided
        if (!address) {
            if (!fetchByOwner) {
                throw new Error('fetchByOwner is required');
            }
            address = getAssociatedTokenAddressSync(
                fetchByOwner.mint,
                fetchByOwner.owner,
                false,
                programId,
                getAtaProgramId(programId),
            );
        }

        const info = await rpc.getAccountInfo(address, commitment);
        if (!info) {
            throw new TokenAccountNotFoundError();
        }

        const account = unpackAccountSPL(address, info, programId);

        const type: TokenAccountSource['type'] = programId.equals(
            TOKEN_PROGRAM_ID,
        )
            ? 'spl-onchain'
            : 'token2022-onchain';

        return {
            accountInfo: info,
            parsed: account,
            isCold: false,
            loadContext: undefined,
            _sources: [
                {
                    type,
                    address,
                    amount: account.amount,
                    accountInfo: info,
                    parsed: account,
                },
            ],
            _needsConsolidation: false,
            _hasDelegate: account.delegate !== null,
            _anyFrozen: account.isFrozen,
        };
    }

    throw new Error(`Unsupported program ID: ${programId.toBase58()}`);
}
