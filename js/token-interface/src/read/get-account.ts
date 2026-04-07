import { AccountInfo, Commitment, PublicKey } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    unpackAccount as unpackAccountSPL,
    TokenAccountNotFoundError,
    TokenInvalidAccountOwnerError,
    getAssociatedTokenAddressSync,
    AccountState,
    Account,
} from '@solana/spl-token';
import {
    Rpc,
    LIGHT_TOKEN_PROGRAM_ID,
    MerkleContext,
    CompressedAccountWithMerkleContext,
    assertV2Enabled,
} from '@lightprotocol/stateless.js';
import { Buffer } from 'buffer';
import BN from 'bn.js';
import { getAtaProgramId, checkAtaAddress } from './ata-utils';
import { ERR_FETCH_BY_OWNER_REQUIRED } from '../errors';

export const TokenAccountSourceType = {
    Spl: 'spl',
    Token2022: 'token2022',
    SplCold: 'spl-cold',
    Token2022Cold: 'token2022-cold',
    LightTokenHot: 'light-token-hot',
    LightTokenCold: 'light-token-cold',
} as const;

export type TokenAccountSourceTypeValue =
    (typeof TokenAccountSourceType)[keyof typeof TokenAccountSourceType];

/** Cold (compressed) source types. Used for load/decompress and isCold. */
export const COLD_SOURCE_TYPES: ReadonlySet<TokenAccountSourceTypeValue> =
    new Set([
        TokenAccountSourceType.LightTokenCold,
        TokenAccountSourceType.SplCold,
        TokenAccountSourceType.Token2022Cold,
    ]);

function isColdSourceType(type: TokenAccountSourceTypeValue): boolean {
    return COLD_SOURCE_TYPES.has(type);
}

/** @internal */
export interface TokenAccountSource {
    type: TokenAccountSourceTypeValue;
    address: PublicKey;
    amount: bigint;
    accountInfo: AccountInfo<Buffer>;
    loadContext?: MerkleContext;
    parsed: Account;
}

export interface AccountView {
    accountInfo: AccountInfo<Buffer>;
    parsed: Account;
    isCold: boolean;
    loadContext?: MerkleContext;
    _sources?: TokenAccountSource[];
    _needsConsolidation?: boolean;
    _hasDelegate?: boolean;
    _anyFrozen?: boolean;
    /** True when fetched via getAtaView */
    _isAta?: boolean;
    /** Associated token account owner - set by getAtaView */
    _owner?: PublicKey;
    /** Associated token account mint - set by getAtaView */
    _mint?: PublicKey;
}

type CompressedByOwnerResult = Awaited<
    ReturnType<Rpc['getCompressedTokenAccountsByOwner']>
>;

function toErrorMessage(error: unknown): string {
    if (error instanceof Error) return error.message;
    return String(error);
}

function throwRpcFetchFailure(context: string, error: unknown): never {
    throw new Error(`${context}: ${toErrorMessage(error)}`);
}

function throwIfUnexpectedRpcErrors(
    context: string,
    unexpectedErrors: unknown[],
): void {
    if (unexpectedErrors.length > 0) {
        throwRpcFetchFailure(context, unexpectedErrors[0]);
    }
}

export type FrozenOperation =
    | 'load'
    | 'transfer'
    | 'unwrap'
    | 'approve'
    | 'revoke';

export function checkNotFrozen(
    iface: AccountView,
    operation: FrozenOperation,
): void {
    if (iface._anyFrozen) {
        throw new Error(
            `Account is frozen. One or more sources (hot or cold) are frozen; ${operation} is not allowed.`,
        );
    }
}

/** @internal */
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
    } catch {
        return null;
    }
}

/**
 * Known extension data sizes by Borsh enum discriminator.
 * undefined = variable-length (cannot skip without full parsing).
 * @internal
 */
const EXTENSION_DATA_SIZES: Record<number, number | undefined> = {
    0: 0,
    1: 0,
    2: 0,
    3: 0,
    4: 0,
    5: 0,
    6: 0,
    7: 0,
    8: 0,
    9: 0,
    10: 0,
    11: 0,
    12: 0,
    13: 0,
    14: 0,
    15: 0,
    16: 0,
    17: 0,
    18: 0,
    19: undefined, // TokenMetadata (variable)
    20: 0,
    21: 0,
    22: 0,
    23: 0,
    24: 0,
    25: 0,
    26: 0,
    27: 0, // PausableAccountExtension (unit struct)
    28: 0, // PermanentDelegateAccountExtension (unit struct)
    29: 8, // TransferFeeAccountExtension (u64)
    30: 1, // TransferHookAccountExtension (u8)
    31: 17, // CompressedOnlyExtension (u64 + u64 + u8)
    32: undefined, // CompressibleExtension (variable)
};

const COMPRESSED_ONLY_DISCRIMINATOR = 31;

/**
 * Extract delegated_amount from CompressedOnly extension in Borsh-serialized
 * TLV data (Vec<ExtensionStruct>).
 * @internal
 */
function extractDelegatedAmountFromTlv(tlv: Buffer | null): bigint | null {
    if (!tlv || tlv.length < 5) return null;

    try {
        let offset = 0;
        const vecLen = tlv.readUInt32LE(offset);
        offset += 4;

        for (let i = 0; i < vecLen; i++) {
            if (offset >= tlv.length) return null;

            const discriminator = tlv[offset];
            offset += 1;

            if (discriminator === COMPRESSED_ONLY_DISCRIMINATOR) {
                if (offset + 8 > tlv.length) return null;
                // delegated_amount is the first u64 field
                const lo = BigInt(tlv.readUInt32LE(offset));
                const hi = BigInt(tlv.readUInt32LE(offset + 4));
                return lo | (hi << BigInt(32));
            }

            const size = EXTENSION_DATA_SIZES[discriminator];
            if (size === undefined) return null;
            offset += size;
        }
    } catch {
        return null;
    }

    return null;
}

/** @internal */
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
    // Determine delegatedAmount for compressed TokenData:
    // 1. If CompressedOnly extension present in TLV, use its delegated_amount
    // 2. If delegate is set (regular compressed approve), the entire compressed
    //    account's amount is the delegation (change goes to a separate account)
    // 3. Otherwise, 0
    let delegatedAmount = BigInt(0);
    const extensionDelegatedAmount = extractDelegatedAmountFromTlv(
        tokenData.tlv,
    );
    if (extensionDelegatedAmount !== null) {
        delegatedAmount = extensionDelegatedAmount;
    } else if (tokenData.delegate) {
        delegatedAmount = BigInt(tokenData.amount.toString());
    }

    return {
        address,
        mint: tokenData.mint,
        owner: tokenData.owner,
        amount: BigInt(tokenData.amount.toString()),
        delegate: tokenData.delegate,
        delegatedAmount,
        isInitialized: tokenData.state !== AccountState.Uninitialized,
        isFrozen: tokenData.state === AccountState.Frozen,
        isNative: false,
        rentExemptReserve: null,
        closeAuthority: null,
        tlvData: tokenData.tlv ? Buffer.from(tokenData.tlv) : Buffer.alloc(0),
    };
}

function requireCompressedAccountData(
    compressedAccount: CompressedAccountWithMerkleContext,
): NonNullable<CompressedAccountWithMerkleContext['data']> {
    const data = compressedAccount.data;
    if (!data) {
        throw new Error('Compressed account is missing token data');
    }
    return data;
}

/** Convert compressed account to AccountInfo */
function toAccountInfo(
    compressedAccount: CompressedAccountWithMerkleContext,
): AccountInfo<Buffer> {
    const compressedData = requireCompressedAccountData(compressedAccount);
    const dataDiscriminatorBuffer: Buffer = Buffer.from(
        compressedData.discriminator,
    );
    const dataBuffer: Buffer = Buffer.from(compressedData.data);
    const data: Buffer = Buffer.concat([dataDiscriminatorBuffer, dataBuffer]);

    return {
        executable: false,
        owner: compressedAccount.owner,
        lamports: compressedAccount.lamports.toNumber(),
        data,
        rentEpoch: undefined,
    };
}

/** @internal */
export function parseLightTokenHot(
    address: PublicKey,
    accountInfo: AccountInfo<Buffer>,
): {
    accountInfo: AccountInfo<Buffer>;
    loadContext: undefined;
    parsed: Account;
    isCold: false;
} {
    // Hot light-token accounts use SPL-compatible layout with 4-byte COption tags.
    // unpackAccountSPL correctly parses all fields including delegatedAmount,
    // isNative, and closeAuthority.
    const parsed = unpackAccountSPL(
        address,
        accountInfo,
        LIGHT_TOKEN_PROGRAM_ID,
    );
    return {
        accountInfo,
        loadContext: undefined,
        parsed,
        isCold: false,
    };
}

/** @internal */
export function parseLightTokenCold(
    address: PublicKey,
    compressedAccount: CompressedAccountWithMerkleContext,
): {
    accountInfo: AccountInfo<Buffer>;
    loadContext: MerkleContext;
    parsed: Account;
    isCold: true;
} {
    const parsed = parseTokenData(
        requireCompressedAccountData(compressedAccount).data,
    );
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
 * Retrieve associated token account for a given owner and mint.
 *
 * @param rpc                RPC connection
 * @param ata                Associated token address
 * @param owner              Owner public key
 * @param mint               Mint public key
 * @param commitment         Optional commitment level
 * @param programId          Optional program ID
 * @param wrap               Include SPL/T22 balances (default: false)
 * @param allowOwnerOffCurve Allow owner to be off-curve (PDA)
 * @returns AccountView with associated token account metadata
 */
export async function getAtaView(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
    wrap = false,
    allowOwnerOffCurve = false,
): Promise<AccountView> {
    assertV2Enabled();

    // Invariant: ata MUST match a valid derivation from mint+owner.
    // Hot path: if programId provided, only validate against that program.
    // For wrap=true, additionally require light-token associated token account.
    const validation = checkAtaAddress(
        ata,
        mint,
        owner,
        programId,
        allowOwnerOffCurve,
    );

    if (wrap && validation.type !== 'light-token') {
        throw new Error(
            `For wrap=true, ata must be the light-token ATA. Got ${validation.type} ATA instead.`,
        );
    }

    // Pass both ata address AND fetchByOwner for proper lookups:
    // - address is used for on-chain account fetching
    // - fetchByOwner is used for light-token lookup by owner+mint
    const result = await _getAccountView(
        rpc,
        ata,
        commitment,
        programId,
        {
            owner,
            mint,
        },
        wrap,
    );
    result._isAta = true;
    result._owner = owner;
    result._mint = mint;
    return result;
}

/**
 * @internal
 */
async function _getAccountView(
    rpc: Rpc,
    address: PublicKey | undefined,
    commitment: Commitment | undefined,
    programId: PublicKey | undefined,
    fetchByOwner: { owner: PublicKey; mint: PublicKey } | undefined,
    wrap: boolean,
): Promise<AccountView> {
    if (!programId) {
        return getUnifiedAccountView(
            rpc,
            address,
            commitment,
            fetchByOwner,
            wrap,
        );
    }

    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        return getLightTokenAccountView(rpc, address, commitment, fetchByOwner);
    }

    if (
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        return getSplOrToken2022AccountView(
            rpc,
            address,
            commitment,
            programId,
            fetchByOwner,
        );
    }

    throw new TokenInvalidAccountOwnerError();
}

/**
 * @internal
 */
async function _tryFetchSpl(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    parsed: Account;
    isCold: false;
    loadContext: undefined;
} | null> {
    const info = await rpc.getAccountInfo(address, commitment);
    if (!info) {
        return null;
    }
    if (!info.owner.equals(TOKEN_PROGRAM_ID)) {
        throw new TokenInvalidAccountOwnerError();
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
 * @internal
 */
async function _tryFetchToken2022(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    parsed: Account;
    isCold: false;
    loadContext: undefined;
} | null> {
    const info = await rpc.getAccountInfo(address, commitment);
    if (!info) {
        return null;
    }
    if (!info.owner.equals(TOKEN_2022_PROGRAM_ID)) {
        throw new TokenInvalidAccountOwnerError();
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
 * @internal
 */
async function _tryFetchLightTokenHot(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    loadContext: undefined;
    parsed: Account;
    isCold: false;
} | null> {
    const info = await rpc.getAccountInfo(address, commitment);
    if (!info) {
        return null;
    }
    if (!info.owner.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        throw new TokenInvalidAccountOwnerError();
    }
    return parseLightTokenHot(address, info);
}

/** @internal */
async function getUnifiedAccountView(
    rpc: Rpc,
    address: PublicKey | undefined,
    commitment: Commitment | undefined,
    fetchByOwner: { owner: PublicKey; mint: PublicKey } | undefined,
    wrap: boolean,
): Promise<AccountView> {
    if (!address && !fetchByOwner) {
        throw new Error(ERR_FETCH_BY_OWNER_REQUIRED);
    }

    // Canonical address for unified mode is always the light-token associated token account
    let lightTokenAta: PublicKey;
    if (address) {
        lightTokenAta = address;
    } else {
        if (!fetchByOwner) {
            throw new Error(ERR_FETCH_BY_OWNER_REQUIRED);
        }
        lightTokenAta = getAssociatedTokenAddressSync(
            fetchByOwner.mint,
            fetchByOwner.owner,
            true,
            LIGHT_TOKEN_PROGRAM_ID,
            getAtaProgramId(LIGHT_TOKEN_PROGRAM_ID),
        );
    }

    const fetchPromises: Promise<{
        accountInfo: AccountInfo<Buffer>;
        parsed: Account;
        isCold: boolean;
        loadContext?: MerkleContext;
    } | null>[] = [];
    const fetchTypes: TokenAccountSource['type'][] = [];
    const fetchAddresses: PublicKey[] = [];

    // light-token hot
    fetchPromises.push(_tryFetchLightTokenHot(rpc, lightTokenAta, commitment));
    fetchTypes.push(TokenAccountSourceType.LightTokenHot);
    fetchAddresses.push(lightTokenAta);

    // SPL / Token-2022 (only when wrap is enabled)
    if (wrap) {
        // Always derive SPL/T22 addresses from owner+mint, not from the passed
        // light-token address. SPL and T22 associated token accounts are different from light-token associated token accounts.
        if (!fetchByOwner) {
            throw new Error(
                'fetchByOwner is required for wrap=true to derive SPL/T22 addresses',
            );
        }
        const splTokenAta = getAssociatedTokenAddressSync(
            fetchByOwner.mint,
            fetchByOwner.owner,
            true,
            TOKEN_PROGRAM_ID,
            getAtaProgramId(TOKEN_PROGRAM_ID),
        );
        const token2022Ata = getAssociatedTokenAddressSync(
            fetchByOwner.mint,
            fetchByOwner.owner,
            true,
            TOKEN_2022_PROGRAM_ID,
            getAtaProgramId(TOKEN_2022_PROGRAM_ID),
        );

        fetchPromises.push(_tryFetchSpl(rpc, splTokenAta, commitment));
        fetchTypes.push(TokenAccountSourceType.Spl);
        fetchAddresses.push(splTokenAta);

        fetchPromises.push(_tryFetchToken2022(rpc, token2022Ata, commitment));
        fetchTypes.push(TokenAccountSourceType.Token2022);
        fetchAddresses.push(token2022Ata);
    }

    // Fetch ALL cold light-token accounts (not just one) - important for V1/V2 detection
    const coldAccountsPromise = fetchByOwner
        ? rpc.getCompressedTokenAccountsByOwner(fetchByOwner.owner, {
              mint: fetchByOwner.mint,
          })
        : rpc.getCompressedTokenAccountsByOwner(lightTokenAta);

    const hotResults = await Promise.allSettled(fetchPromises);
    const ownerMismatchErrors: TokenInvalidAccountOwnerError[] = [];
    const unexpectedErrors: unknown[] = [];

    let coldResult: Awaited<typeof coldAccountsPromise> | null = null;
    try {
        coldResult = await coldAccountsPromise;
    } catch (error) {
        unexpectedErrors.push(error);
    }

    // collect all successful hot results
    const sources: TokenAccountSource[] = [];

    for (let i = 0; i < hotResults.length; i++) {
        const result = hotResults[i];
        if (result.status === 'fulfilled') {
            const value = result.value;
            if (!value) {
                continue;
            }
            sources.push({
                type: fetchTypes[i],
                address: fetchAddresses[i],
                amount: value.parsed.amount,
                accountInfo: value.accountInfo,
                loadContext: value.loadContext,
                parsed: value.parsed,
            });
        } else if (result.reason instanceof TokenInvalidAccountOwnerError) {
            ownerMismatchErrors.push(result.reason);
        } else {
            unexpectedErrors.push(result.reason);
        }
    }

    // Add ALL cold light-token accounts (handles both V1 and V2)
    if (coldResult) {
        for (const item of coldResult.items) {
            const compressedAccount = item.compressedAccount;
            if (
                compressedAccount &&
                compressedAccount.data &&
                compressedAccount.data.data.length > 0 &&
                compressedAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)
            ) {
                const parsed = parseLightTokenCold(
                    lightTokenAta,
                    compressedAccount,
                );
                sources.push({
                    type: TokenAccountSourceType.LightTokenCold,
                    address: lightTokenAta,
                    amount: parsed.parsed.amount,
                    accountInfo: parsed.accountInfo,
                    loadContext: parsed.loadContext,
                    parsed: parsed.parsed,
                });
            }
        }
    }

    throwIfUnexpectedRpcErrors(
        'Failed to fetch token account data from RPC',
        unexpectedErrors,
    );

    // account not found
    if (sources.length === 0) {
        if (ownerMismatchErrors.length > 0) {
            throw ownerMismatchErrors[0];
        }
        throw new TokenAccountNotFoundError();
    }

    // priority order: light-token hot > light-token cold > SPL/T22
    const priority: TokenAccountSource['type'][] = [
        TokenAccountSourceType.LightTokenHot,
        TokenAccountSourceType.LightTokenCold,
        TokenAccountSourceType.Spl,
        TokenAccountSourceType.Token2022,
    ];

    sources.sort((a, b) => {
        const aIdx = priority.indexOf(a.type);
        const bIdx = priority.indexOf(b.type);
        return aIdx - bIdx;
    });

    return buildAccountViewFromSources(sources, lightTokenAta);
}

/** @internal */
async function getLightTokenAccountView(
    rpc: Rpc,
    address: PublicKey | undefined,
    commitment: Commitment | undefined,
    fetchByOwner?: { owner: PublicKey; mint: PublicKey },
): Promise<AccountView> {
    // Derive address if not provided
    if (!address) {
        if (!fetchByOwner) {
            throw new Error(ERR_FETCH_BY_OWNER_REQUIRED);
        }
        address = getAssociatedTokenAddressSync(
            fetchByOwner.mint,
            fetchByOwner.owner,
            true,
            LIGHT_TOKEN_PROGRAM_ID,
            getAtaProgramId(LIGHT_TOKEN_PROGRAM_ID),
        );
    }

    const [onchainResult, compressedResult] = await Promise.allSettled([
        rpc.getAccountInfo(address, commitment),
        // Fetch compressed: by owner+mint for associated token accounts, by address for non-ATAs
        fetchByOwner
            ? rpc.getCompressedTokenAccountsByOwner(fetchByOwner.owner, {
                  mint: fetchByOwner.mint,
              })
            : rpc.getCompressedTokenAccountsByOwner(address),
    ]);
    const unexpectedErrors: unknown[] = [];
    const ownerMismatchErrors: TokenInvalidAccountOwnerError[] = [];

    const onchainAccount =
        onchainResult.status === 'fulfilled' ? onchainResult.value : null;
    if (onchainResult.status === 'rejected') {
        unexpectedErrors.push(onchainResult.reason);
    }
    const compressedAccounts =
        compressedResult.status === 'fulfilled'
            ? compressedResult.value.items.map(item => item.compressedAccount)
            : [];
    if (compressedResult.status === 'rejected') {
        unexpectedErrors.push(compressedResult.reason);
    }

    const sources: TokenAccountSource[] = [];

    // Collect light-token associated token account (hot balance)
    if (onchainAccount) {
        if (!onchainAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)) {
            ownerMismatchErrors.push(new TokenInvalidAccountOwnerError());
        } else {
            const parsed = parseLightTokenHot(address, onchainAccount);
            sources.push({
                type: TokenAccountSourceType.LightTokenHot,
                address,
                amount: parsed.parsed.amount,
                accountInfo: onchainAccount,
                parsed: parsed.parsed,
            });
        }
    }

    // Collect compressed light-token accounts (cold balance)
    for (const compressedAccount of compressedAccounts) {
        if (
            compressedAccount &&
            compressedAccount.data &&
            compressedAccount.data.data.length > 0 &&
            compressedAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)
        ) {
            const parsed = parseLightTokenCold(address, compressedAccount);
            sources.push({
                type: TokenAccountSourceType.LightTokenCold,
                address,
                amount: parsed.parsed.amount,
                accountInfo: parsed.accountInfo,
                loadContext: parsed.loadContext,
                parsed: parsed.parsed,
            });
        }
    }

    throwIfUnexpectedRpcErrors(
        'Failed to fetch token account data from RPC',
        unexpectedErrors,
    );

    if (sources.length === 0) {
        if (ownerMismatchErrors.length > 0) {
            throw ownerMismatchErrors[0];
        }
        throw new TokenAccountNotFoundError();
    }

    // Priority: hot > cold
    sources.sort((a, b) => {
        if (a.type === 'light-token-hot' && b.type === 'light-token-cold')
            return -1;
        if (a.type === 'light-token-cold' && b.type === 'light-token-hot')
            return 1;
        return 0;
    });

    return buildAccountViewFromSources(sources, address);
}

/** @internal */
async function getSplOrToken2022AccountView(
    rpc: Rpc,
    address: PublicKey | undefined,
    commitment: Commitment | undefined,
    programId: PublicKey,
    fetchByOwner?: { owner: PublicKey; mint: PublicKey },
): Promise<AccountView> {
    if (!address) {
        if (!fetchByOwner) {
            throw new Error(ERR_FETCH_BY_OWNER_REQUIRED);
        }
        address = getAssociatedTokenAddressSync(
            fetchByOwner.mint,
            fetchByOwner.owner,
            true,
            programId,
            getAtaProgramId(programId),
        );
    }

    const hotType: TokenAccountSource['type'] = programId.equals(
        TOKEN_PROGRAM_ID,
    )
        ? TokenAccountSourceType.Spl
        : TokenAccountSourceType.Token2022;

    const coldType: TokenAccountSource['type'] = programId.equals(
        TOKEN_PROGRAM_ID,
    )
        ? TokenAccountSourceType.SplCold
        : TokenAccountSourceType.Token2022Cold;

    // Fetch hot and cold in parallel (neither is required individually)
    const [hotResult, coldResult] = await Promise.allSettled([
        rpc.getAccountInfo(address, commitment),
        fetchByOwner
            ? rpc.getCompressedTokenAccountsByOwner(fetchByOwner.owner, {
                  mint: fetchByOwner.mint,
              })
            : Promise.resolve(null as CompressedByOwnerResult | null),
    ]);

    const sources: TokenAccountSource[] = [];
    const unexpectedErrors: unknown[] = [];
    const ownerMismatchErrors: TokenInvalidAccountOwnerError[] = [];

    const hotInfo = hotResult.status === 'fulfilled' ? hotResult.value : null;
    if (hotResult.status === 'rejected')
        unexpectedErrors.push(hotResult.reason);
    const coldAccounts =
        coldResult.status === 'fulfilled' ? coldResult.value : null;
    if (coldResult.status === 'rejected')
        unexpectedErrors.push(coldResult.reason);

    // Hot SPL/T22 account (may not exist)
    if (hotInfo) {
        if (!hotInfo.owner.equals(programId)) {
            ownerMismatchErrors.push(new TokenInvalidAccountOwnerError());
        } else {
            try {
                const account = unpackAccountSPL(address, hotInfo, programId);
                sources.push({
                    type: hotType,
                    address,
                    amount: account.amount,
                    accountInfo: hotInfo,
                    parsed: account,
                });
            } catch (error) {
                unexpectedErrors.push(error);
            }
        }
    }

    // Cold (compressed) accounts
    for (const item of coldAccounts?.items ?? []) {
        const compressedAccount = item.compressedAccount;
        if (
            compressedAccount &&
            compressedAccount.data &&
            compressedAccount.data.data.length > 0 &&
            compressedAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)
        ) {
            const parsedCold = parseLightTokenCold(address, compressedAccount);
            sources.push({
                type: coldType,
                address,
                amount: parsedCold.parsed.amount,
                accountInfo: parsedCold.accountInfo,
                loadContext: parsedCold.loadContext,
                parsed: parsedCold.parsed,
            });
        }
    }

    throwIfUnexpectedRpcErrors(
        'Failed to fetch token account data from RPC',
        unexpectedErrors,
    );

    if (sources.length === 0) {
        if (ownerMismatchErrors.length > 0) {
            throw ownerMismatchErrors[0];
        }
        throw new TokenAccountNotFoundError();
    }

    return buildAccountViewFromSources(sources, address);
}

/** @internal */
function buildAccountViewFromSources(
    sources: TokenAccountSource[],
    canonicalAddress: PublicKey,
): AccountView {
    const totalAmount = sources.reduce(
        (sum, src) => sum + src.amount,
        BigInt(0),
    );

    const primarySource = sources[0];

    const hasDelegate = sources.some(src => src.parsed.delegate !== null);
    const anyFrozen = sources.some(src => src.parsed.isFrozen);
    const hasColdSource = sources.some(src => isColdSourceType(src.type));
    const needsConsolidation = sources.length > 1;
    const delegatedContribution = (src: TokenAccountSource): bigint => {
        const delegated = src.parsed.delegatedAmount ?? src.amount;
        return src.amount < delegated ? src.amount : delegated;
    };

    const sumForDelegate = (
        candidate: PublicKey,
        scope: (src: TokenAccountSource) => boolean,
    ): bigint =>
        sources.reduce((sum, src) => {
            if (!scope(src)) return sum;
            const delegate = src.parsed.delegate;
            if (!delegate || !delegate.equals(candidate)) return sum;
            return sum + delegatedContribution(src);
        }, BigInt(0));

    const hotDelegatedSource = sources.find(
        src => !isColdSourceType(src.type) && src.parsed.delegate !== null,
    );
    const coldDelegatedSources = sources.filter(
        src => isColdSourceType(src.type) && src.parsed.delegate !== null,
    );

    let canonicalDelegate: PublicKey | null = null;
    let canonicalDelegatedAmount = BigInt(0);

    if (hotDelegatedSource?.parsed.delegate) {
        // If any hot source is delegated, it always determines canonical delegate.
        // Cold delegates only contribute when they match this hot delegate.
        canonicalDelegate = hotDelegatedSource.parsed.delegate;
        canonicalDelegatedAmount = sumForDelegate(
            canonicalDelegate,
            () => true,
        );
    } else if (coldDelegatedSources.length > 0) {
        // No hot delegate: canonical delegate is taken from the most recent
        // delegated cold source in source order (source[0] is most recent).
        const firstColdDelegate = coldDelegatedSources[0].parsed.delegate;
        if (firstColdDelegate) {
            canonicalDelegate = firstColdDelegate;
            canonicalDelegatedAmount = sumForDelegate(canonicalDelegate, src =>
                isColdSourceType(src.type),
            );
        }
    }

    const unifiedAccount: Account = {
        ...primarySource.parsed,
        address: canonicalAddress,
        amount: totalAmount,
        // Synthetic ATA view models post-load state; any cold source implies initialized.
        isInitialized: primarySource.parsed.isInitialized || hasColdSource,
        delegate: canonicalDelegate,
        delegatedAmount: canonicalDelegatedAmount,
        ...(anyFrozen ? { state: AccountState.Frozen, isFrozen: true } : {}),
    };

    return {
        accountInfo: primarySource.accountInfo,
        parsed: unifiedAccount,
        isCold: isColdSourceType(primarySource.type),
        loadContext: primarySource.loadContext,
        _sources: sources,
        _needsConsolidation: needsConsolidation,
        _hasDelegate: hasDelegate,
        _anyFrozen: anyFrozen,
    };
}

/**
 * Spendable amount for a given authority (owner or delegate).
 * - If authority equals the ATA owner: full parsed.amount.
 * - If authority is the canonical delegate: parsed.delegatedAmount (bounded by parsed.amount).
 * - Otherwise: 0.
 * @internal
 */
function spendableAmountForAuthority(
    iface: AccountView,
    authority: PublicKey,
): bigint {
    const owner = iface._owner;
    if (owner && authority.equals(owner)) {
        return iface.parsed.amount;
    }
    const delegate = iface.parsed.delegate;
    if (delegate && authority.equals(delegate)) {
        const delegated = iface.parsed.delegatedAmount ?? BigInt(0);
        return delegated < iface.parsed.amount
            ? delegated
            : iface.parsed.amount;
    }
    return BigInt(0);
}

/**
 * Whether the given authority can sign for this ATA (owner or canonical delegate).
 * @internal
 */
export function isAuthorityForAccount(
    iface: AccountView,
    authority: PublicKey,
): boolean {
    const owner = iface._owner;
    if (owner && authority.equals(owner)) return true;
    const delegate = iface.parsed.delegate;
    return delegate !== null && authority.equals(delegate);
}

/**
 * @internal
 * Canonical authority projection for owner/delegate checks.
 */
export function filterAccountForAuthority(
    iface: AccountView,
    authority: PublicKey,
): AccountView {
    const owner = iface._owner;
    if (owner && authority.equals(owner)) {
        return iface;
    }
    const spendable = spendableAmountForAuthority(iface, authority);
    const canonicalDelegate = iface.parsed.delegate;
    if (
        spendable === BigInt(0) ||
        canonicalDelegate === null ||
        !authority.equals(canonicalDelegate)
    ) {
        return {
            ...iface,
            _sources: [],
            _needsConsolidation: false,
            parsed: { ...iface.parsed, amount: BigInt(0) },
        };
    }
    const sources = iface._sources ?? [];
    const filtered = sources.filter(
        src =>
            src.parsed.delegate !== null &&
            src.parsed.delegate.equals(canonicalDelegate),
    );
    const primary = filtered[0];
    return {
        ...iface,
        ...(primary
            ? {
                  accountInfo: primary.accountInfo,
                  isCold: isColdSourceType(primary.type),
                  loadContext: primary.loadContext,
              }
            : {}),
        _sources: filtered,
        _needsConsolidation: filtered.length > 1,
        parsed: {
            ...iface.parsed,
            amount: spendable,
        },
    };
}
