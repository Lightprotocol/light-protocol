import { AccountInfo, Commitment, PublicKey } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    unpackAccount as unpackAccountSPL,
    TokenAccountNotFoundError,
    getAssociatedTokenAddressSync,
    AccountState,
    Account,
} from '@solana/spl-token';
import {
    Rpc,
    LIGHT_TOKEN_PROGRAM_ID,
    MerkleContext,
    CompressedAccountWithMerkleContext,
    deriveAddressV2,
    bn,
    getDefaultAddressTreeInfo,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import { Buffer } from 'buffer';
import BN from 'bn.js';
import { getAtaProgramId, checkAtaAddress } from './ata-utils';
export { Account, AccountState } from '@solana/spl-token';
export { ParsedTokenAccount } from '@lightprotocol/stateless.js';

export const TokenAccountSourceType = {
    Spl: 'spl',
    Token2022: 'token2022',
    SplCold: 'spl-cold',
    Token2022Cold: 'token2022-cold',
    CTokenHot: 'ctoken-hot',
    CTokenCold: 'ctoken-cold',
} as const;

export type TokenAccountSourceTypeValue =
    (typeof TokenAccountSourceType)[keyof typeof TokenAccountSourceType];

/** @internal */
export interface TokenAccountSource {
    type: TokenAccountSourceTypeValue;
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
    /** True when fetched via getAtaInterface */
    _isAta?: boolean;
    /** ATA owner - set by getAtaInterface */
    _owner?: PublicKey;
    /** ATA mint - set by getAtaInterface */
    _mint?: PublicKey;
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
    } catch (error) {
        console.error('Token data parsing error:', error);
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

/** Convert compressed account to AccountInfo */
export function toAccountInfo(
    compressedAccount: CompressedAccountWithMerkleContext,
): AccountInfo<Buffer> {
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

/** @internal */
export function parseCTokenHot(
    address: PublicKey,
    accountInfo: AccountInfo<Buffer>,
): {
    accountInfo: AccountInfo<Buffer>;
    loadContext: undefined;
    parsed: Account;
    isCold: false;
} {
    // Hot c-token accounts use SPL-compatible layout with 4-byte COption tags.
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
export function parseCTokenCold(
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
 * Retrieve information about a token account of SPL/T22/c-token.
 *
 * @param rpc        RPC connection to use
 * @param address    Token account address
 * @param commitment Desired level of commitment for querying the state
 * @param programId  Token program ID. If not provided, tries all programs concurrently.
 *
 * @return Token account information with compression context if applicable
 */
export async function getAccountInterface(
    rpc: Rpc,
    address: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
): Promise<AccountInterface> {
    assertBetaEnabled();

    return _getAccountInterface(rpc, address, commitment, programId, undefined);
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
 * @returns AccountInterface with ATA metadata
 */
export async function getAtaInterface(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
    wrap = false,
    allowOwnerOffCurve = false,
): Promise<AccountInterface> {
    assertBetaEnabled();

    // Invariant: ata MUST match a valid derivation from mint+owner.
    // Hot path: if programId provided, only validate against that program.
    // For wrap=true, additionally require c-token ATA.
    const validation = checkAtaAddress(
        ata,
        mint,
        owner,
        programId,
        allowOwnerOffCurve,
    );

    if (wrap && validation.type !== 'ctoken') {
        throw new Error(
            `For wrap=true, ata must be the c-token ATA. Got ${validation.type} ATA instead.`,
        );
    }

    // Pass both ata address AND fetchByOwner for proper lookups:
    // - address is used for on-chain account fetching
    // - fetchByOwner is used for compressed token lookup by owner+mint
    const result = await _getAccountInterface(
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
async function _tryFetchSpl(
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
 * @internal
 */
async function _tryFetchCTokenHot(
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
    if (!info || !info.owner.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        throw new Error('Not a CTOKEN onchain account');
    }
    return parseCTokenHot(address, info);
}

/**
 * @internal
 */
async function _tryFetchCTokenColdByOwner(
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
    if (!compressedAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        throw new Error('Invalid owner for compressed token');
    }
    return parseCTokenCold(ataAddress, compressedAccount);
}

/**
 * @internal
 * Fetch compressed token account by deriving its compressed address from the on-chain address.
 * Uses deriveAddressV2(address, addressTree, LIGHT_TOKEN_PROGRAM_ID) to get the compressed address.
 *
 * Note: This only works for accounts that were **compressed from on-chain** (via compress_accounts_idempotent).
 * For tokens minted compressed (via mintTo), use getAtaInterface with owner+mint instead.
 */
async function _tryFetchCTokenColdByAddress(
    rpc: Rpc,
    address: PublicKey,
): Promise<{
    accountInfo: AccountInfo<Buffer>;
    loadContext: MerkleContext;
    parsed: Account;
    isCold: true;
}> {
    // Derive compressed address from on-chain token account address
    const addressTree = getDefaultAddressTreeInfo().tree;
    const compressedAddress = deriveAddressV2(
        address.toBytes(),
        addressTree,
        LIGHT_TOKEN_PROGRAM_ID,
    );

    // Fetch by derived compressed address
    const compressedAccount = await rpc.getCompressedAccount(
        bn(compressedAddress.toBytes()),
    );

    if (!compressedAccount?.data?.data.length) {
        throw new Error(
            'Compressed token account not found at derived address. ' +
                'Note: getAccountInterface only finds compressed accounts that were ' +
                'compressed from on-chain (via compress_accounts_idempotent). ' +
                'For tokens minted compressed (via mintTo), use getAtaInterface with owner+mint.',
        );
    }
    if (!compressedAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        throw new Error('Invalid owner for compressed token');
    }
    return parseCTokenCold(address, compressedAccount);
}

/**
 * @internal
 * Retrieve information about a token account SPL/T22/c-token.
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
    wrap = false,
): Promise<AccountInterface> {
    // At least one of address or fetchByOwner is required.
    // Both can be provided: address for on-chain lookup, fetchByOwner for
    // compressed token lookup by owner+mint (useful for PDA owners where
    // address derivation might not work with standard allowOwnerOffCurve=false).
    if (!address && !fetchByOwner) {
        throw new Error('One of address or fetchByOwner is required');
    }

    // Unified mode (auto-detect: c-token + optional SPL/T22)
    if (!programId) {
        return getUnifiedAccountInterface(
            rpc,
            address,
            commitment,
            fetchByOwner,
            wrap,
        );
    }

    // c-token-only mode
    if (programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        return getCTokenAccountInterface(
            rpc,
            address,
            commitment,
            fetchByOwner,
        );
    }

    // SPL / Token-2022 only
    if (
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        return getSplOrToken2022AccountInterface(
            rpc,
            address,
            commitment,
            programId,
            fetchByOwner,
        );
    }

    throw new Error(`Unsupported program ID: ${programId.toBase58()}`);
}

async function getUnifiedAccountInterface(
    rpc: Rpc,
    address: PublicKey | undefined,
    commitment: Commitment | undefined,
    fetchByOwner: { owner: PublicKey; mint: PublicKey } | undefined,
    wrap: boolean,
): Promise<AccountInterface> {
    // Canonical address for unified mode is always the c-token ATA
    const cTokenAta =
        address ??
        getAssociatedTokenAddressSync(
            fetchByOwner!.mint,
            fetchByOwner!.owner,
            false,
            LIGHT_TOKEN_PROGRAM_ID,
            getAtaProgramId(LIGHT_TOKEN_PROGRAM_ID),
        );

    const fetchPromises: Promise<{
        accountInfo: AccountInfo<Buffer>;
        parsed: Account;
        isCold: boolean;
        loadContext?: MerkleContext;
    }>[] = [];
    const fetchTypes: TokenAccountSource['type'][] = [];
    const fetchAddresses: PublicKey[] = [];

    // c-token hot
    fetchPromises.push(_tryFetchCTokenHot(rpc, cTokenAta, commitment));
    fetchTypes.push(TokenAccountSourceType.CTokenHot);
    fetchAddresses.push(cTokenAta);

    // SPL / Token-2022 (only when wrap is enabled)
    if (wrap) {
        // Always derive SPL/T22 addresses from owner+mint, not from the passed
        // c-token address. SPL and T22 ATAs are different from c-token ATAs.
        if (!fetchByOwner) {
            throw new Error(
                'fetchByOwner is required for wrap=true to derive SPL/T22 addresses',
            );
        }
        const splTokenAta = getAssociatedTokenAddressSync(
            fetchByOwner.mint,
            fetchByOwner.owner,
            false,
            TOKEN_PROGRAM_ID,
            getAtaProgramId(TOKEN_PROGRAM_ID),
        );
        const token2022Ata = getAssociatedTokenAddressSync(
            fetchByOwner.mint,
            fetchByOwner.owner,
            false,
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

    // Fetch ALL cold c-token accounts (not just one) - important for V1/V2 detection
    const coldAccountsPromise = fetchByOwner
        ? rpc.getCompressedTokenAccountsByOwner(fetchByOwner.owner, {
              mint: fetchByOwner.mint,
          })
        : rpc.getCompressedTokenAccountsByOwner(address!);

    const [hotResults, coldResult] = await Promise.all([
        Promise.allSettled(fetchPromises),
        coldAccountsPromise.catch(() => ({ items: [] })),
    ]);

    // collect all successful hot results
    const sources: TokenAccountSource[] = [];

    for (let i = 0; i < hotResults.length; i++) {
        const result = hotResults[i];
        if (result.status === 'fulfilled') {
            const value = result.value;
            sources.push({
                type: fetchTypes[i],
                address: fetchAddresses[i],
                amount: value.parsed.amount,
                accountInfo: value.accountInfo,
                loadContext: value.loadContext,
                parsed: value.parsed,
            });
        }
    }

    // Add ALL cold c-token accounts (handles both V1 and V2)
    for (const item of coldResult.items) {
        const compressedAccount = item.compressedAccount;
        if (
            compressedAccount &&
            compressedAccount.data &&
            compressedAccount.data.data.length > 0 &&
            compressedAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)
        ) {
            const parsed = parseCTokenCold(cTokenAta, compressedAccount);
            sources.push({
                type: TokenAccountSourceType.CTokenCold,
                address: cTokenAta,
                amount: parsed.parsed.amount,
                accountInfo: parsed.accountInfo,
                loadContext: parsed.loadContext,
                parsed: parsed.parsed,
            });
        }
    }

    // account not found
    if (sources.length === 0) {
        throw new TokenAccountNotFoundError();
    }

    // priority order: c-token hot > c-token cold > SPL/T22
    const priority: TokenAccountSource['type'][] = [
        TokenAccountSourceType.CTokenHot,
        TokenAccountSourceType.CTokenCold,
        TokenAccountSourceType.Spl,
        TokenAccountSourceType.Token2022,
    ];

    sources.sort((a, b) => {
        const aIdx = priority.indexOf(a.type);
        const bIdx = priority.indexOf(b.type);
        return aIdx - bIdx;
    });

    return buildAccountInterfaceFromSources(sources, cTokenAta);
}

async function getCTokenAccountInterface(
    rpc: Rpc,
    address: PublicKey | undefined,
    commitment: Commitment | undefined,
    fetchByOwner?: { owner: PublicKey; mint: PublicKey },
): Promise<AccountInterface> {
    // Derive address if not provided
    if (!address) {
        if (!fetchByOwner) {
            throw new Error('fetchByOwner is required');
        }
        address = getAssociatedTokenAddressSync(
            fetchByOwner.mint,
            fetchByOwner.owner,
            false,
            LIGHT_TOKEN_PROGRAM_ID,
            getAtaProgramId(LIGHT_TOKEN_PROGRAM_ID),
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
            ? compressedResult.value.items.map(item => item.compressedAccount)
            : [];

    const sources: TokenAccountSource[] = [];

    // Collect hot (decompressed) c-token account
    if (onchainAccount && onchainAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        const parsed = parseCTokenHot(address, onchainAccount);
        sources.push({
            type: TokenAccountSourceType.CTokenHot,
            address,
            amount: parsed.parsed.amount,
            accountInfo: onchainAccount,
            parsed: parsed.parsed,
        });
    }

    // Collect cold (compressed) c-token accounts
    for (const compressedAccount of compressedAccounts) {
        if (
            compressedAccount &&
            compressedAccount.data &&
            compressedAccount.data.data.length > 0 &&
            compressedAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)
        ) {
            const parsed = parseCTokenCold(address, compressedAccount);
            sources.push({
                type: TokenAccountSourceType.CTokenCold,
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

    // Priority: hot > cold
    sources.sort((a, b) => {
        if (a.type === 'ctoken-hot' && b.type === 'ctoken-cold') return -1;
        if (a.type === 'ctoken-cold' && b.type === 'ctoken-hot') return 1;
        return 0;
    });

    return buildAccountInterfaceFromSources(sources, address);
}

async function getSplOrToken2022AccountInterface(
    rpc: Rpc,
    address: PublicKey | undefined,
    commitment: Commitment | undefined,
    programId: PublicKey,
    fetchByOwner?: { owner: PublicKey; mint: PublicKey },
): Promise<AccountInterface> {
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
    const hotPromise = rpc
        .getAccountInfo(address, commitment)
        .catch(() => null);
    const coldPromise = fetchByOwner
        ? rpc
              .getCompressedTokenAccountsByOwner(fetchByOwner.owner, {
                  mint: fetchByOwner.mint,
              })
              .catch(() => ({ items: [] as any[] }))
        : Promise.resolve({ items: [] as any[] });

    const [hotInfo, coldResult] = await Promise.all([hotPromise, coldPromise]);

    const sources: TokenAccountSource[] = [];

    // Hot SPL/T22 account (may not exist)
    if (hotInfo) {
        try {
            const account = unpackAccountSPL(address, hotInfo, programId);
            sources.push({
                type: hotType,
                address,
                amount: account.amount,
                accountInfo: hotInfo,
                parsed: account,
            });
        } catch {
            // Not a valid SPL/T22 account at this address, skip
        }
    }

    // Cold (compressed) accounts
    for (const item of coldResult.items) {
        const compressedAccount = item.compressedAccount;
        if (
            compressedAccount &&
            compressedAccount.data &&
            compressedAccount.data.data.length > 0 &&
            compressedAccount.owner.equals(LIGHT_TOKEN_PROGRAM_ID)
        ) {
            const parsedCold = parseCTokenCold(address, compressedAccount);
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

    if (sources.length === 0) {
        throw new TokenAccountNotFoundError();
    }

    return buildAccountInterfaceFromSources(sources, address);
}

/** @internal */
export function buildAccountInterfaceFromSources(
    sources: TokenAccountSource[],
    canonicalAddress: PublicKey,
): AccountInterface {
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
        address: canonicalAddress,
        amount: totalAmount,
        ...(anyFrozen ? { state: AccountState.Frozen, isFrozen: true } : {}),
    };

    const coldTypes: TokenAccountSource['type'][] = [
        'ctoken-cold',
        'spl-cold',
        'token2022-cold',
    ];

    return {
        accountInfo: primarySource.accountInfo!,
        parsed: unifiedAccount,
        isCold: coldTypes.includes(primarySource.type),
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
 * - If authority is a delegate: sum over sources where delegate === authority
 *   of min(source.amount, source.delegatedAmount).
 *
 * For compress-and-close accounts (CompressedOnly TLV), decompress carries
 * delegate state to the hot ATA. For approve-style accounts (no TLV), the
 * delegate is set in token data but NOT applied to the hot ATA on decompress.
 * The transfer-interface validates this and errors for approve-style cold
 * sources that require loading.
 */
export function spendableAmountForAuthority(
    iface: AccountInterface,
    authority: PublicKey,
): bigint {
    const owner = iface._owner;
    const sources = iface._sources ?? [];
    if (owner && authority.equals(owner)) {
        return iface.parsed.amount;
    }
    let sum = BigInt(0);
    for (const src of sources) {
        if (src.parsed.delegate && authority.equals(src.parsed.delegate)) {
            const amt = src.amount;
            const delegated = src.parsed.delegatedAmount ?? amt;
            sum += amt < delegated ? amt : delegated;
        }
    }
    return sum;
}

/**
 * Whether the given authority can sign for this ATA (is owner or delegate of at least one source).
 */
export function isAuthorityForInterface(
    iface: AccountInterface,
    authority: PublicKey,
): boolean {
    const owner = iface._owner;
    if (owner && authority.equals(owner)) return true;
    const sources = iface._sources ?? [];
    return sources.some(
        src =>
            src.parsed.delegate !== null &&
            authority.equals(src.parsed.delegate),
    );
}

/**
 * Filter an AccountInterface to only sources the given authority can use (owner or delegate).
 * Preserves _owner, _mint, _isAta. Use for load/transfer when authority is delegate.
 */
export function filterInterfaceForAuthority(
    iface: AccountInterface,
    authority: PublicKey,
): AccountInterface {
    const sources = iface._sources ?? [];
    const owner = iface._owner;
    const filtered = sources.filter(
        src =>
            (owner && authority.equals(owner)) ||
            (src.parsed.delegate !== null &&
                authority.equals(src.parsed.delegate)),
    );
    if (filtered.length === 0) {
        return {
            ...iface,
            _sources: [],
            parsed: { ...iface.parsed, amount: BigInt(0) },
        };
    }
    const spendable = spendableAmountForAuthority(iface, authority);
    const primary = filtered[0];
    const anyFrozen = filtered.some(s => s.parsed.isFrozen);
    return {
        ...iface,
        _sources: filtered,
        accountInfo: primary.accountInfo!,
        parsed: {
            ...primary.parsed,
            address: iface.parsed.address,
            amount: spendable,
            ...(anyFrozen
                ? { state: AccountState.Frozen, isFrozen: true }
                : {}),
        },
        isCold: ['ctoken-cold', 'spl-cold', 'token2022-cold'].includes(
            primary.type,
        ),
        loadContext: primary.loadContext,
        _needsConsolidation: filtered.length > 1,
        _hasDelegate: filtered.some(s => s.parsed.delegate !== null),
        _anyFrozen: anyFrozen,
    };
}
