/**
 * Light Token Client Indexer
 *
 * Minimal indexer client for fetching compressed accounts and validity proofs.
 * Implements the core methods needed for the AccountInterface pattern.
 */

import { address as createAddress, type Address } from '@solana/addresses';
import { getBase58Decoder, getBase58Encoder } from '@solana/codecs';

import {
    type CompressedAccount,
    type CompressedTokenAccount,
    type ValidityProofWithContext,
    type GetCompressedTokenAccountsOptions,
    type IndexerResponse,
    type ItemsWithCursor,
    type AddressWithTree,
    type TreeInfo,
    type TokenData,
    type CompressedAccountData,
    type AccountProofInputs,
    type AddressProofInputs,
    type RootIndex,
    TreeType,
    AccountState,
    IndexerError,
    IndexerErrorCode,
    assertV2Tree,
} from '@lightprotocol/token-sdk';

// ============================================================================
// INTERFACES
// ============================================================================

/**
 * Light indexer interface.
 *
 * Provides the minimum methods required for fetching compressed accounts
 * and validity proofs needed for token operations.
 */
export interface LightIndexer {
    /**
     * Fetch a compressed account by its address.
     *
     * @param address - 32-byte compressed account address
     * @returns The compressed account or null if not found
     */
    getCompressedAccount(
        address: Uint8Array,
    ): Promise<IndexerResponse<CompressedAccount | null>>;

    /**
     * Fetch a compressed account by its hash.
     *
     * @param hash - 32-byte account hash
     * @returns The compressed account or null if not found
     */
    getCompressedAccountByHash(
        hash: Uint8Array,
    ): Promise<IndexerResponse<CompressedAccount | null>>;

    /**
     * Fetch compressed token accounts by owner.
     *
     * @param owner - Owner address
     * @param options - Optional filters and pagination
     * @returns Paginated list of token accounts
     */
    getCompressedTokenAccountsByOwner(
        owner: Address,
        options?: GetCompressedTokenAccountsOptions,
    ): Promise<IndexerResponse<ItemsWithCursor<CompressedTokenAccount>>>;

    /**
     * Fetch multiple compressed accounts by their addresses.
     *
     * @param addresses - Array of 32-byte addresses
     * @returns Array of compressed accounts (null for not found)
     */
    getMultipleCompressedAccounts(
        addresses: Uint8Array[],
    ): Promise<IndexerResponse<(CompressedAccount | null)[]>>;

    /**
     * Fetch a validity proof for the given account hashes and new addresses.
     *
     * @param hashes - Account hashes to prove existence
     * @param newAddresses - New addresses to prove uniqueness (optional)
     * @returns Validity proof with context
     */
    getValidityProof(
        hashes: Uint8Array[],
        newAddresses?: AddressWithTree[],
    ): Promise<IndexerResponse<ValidityProofWithContext>>;
}

// ============================================================================
// PHOTON INDEXER IMPLEMENTATION
// ============================================================================

/**
 * JSON-RPC request structure.
 */
interface JsonRpcRequest {
    jsonrpc: '2.0';
    id: string;
    method: string;
    params: unknown;
}

/**
 * JSON-RPC response structure.
 */
interface JsonRpcResponse<T> {
    jsonrpc: '2.0';
    id: string;
    result?: {
        context: { slot: number };
        value: T;
    };
    error?: {
        code: number;
        message: string;
        data?: unknown;
    };
}

/**
 * Photon indexer client.
 *
 * Implements the LightIndexer interface using the Photon API.
 */
export class PhotonIndexer implements LightIndexer {
    private requestId = 0;
    // base58Encoder: string -> Uint8Array (for decoding base58 strings FROM API)
    private readonly base58Encoder = getBase58Encoder();
    // base58Decoder: Uint8Array -> string (for encoding bytes TO base58 for API)
    private readonly base58Decoder = getBase58Decoder();

    /**
     * Create a new PhotonIndexer.
     *
     * @param endpoint - Photon API endpoint URL
     */
    constructor(private readonly endpoint: string) {}

    async getCompressedAccount(
        address: Uint8Array,
    ): Promise<IndexerResponse<CompressedAccount | null>> {
        const addressB58 = this.bytesToBase58(address);
        const response = await this.rpcCall<PhotonAccountV2 | null>(
            'getCompressedAccountV2',
            { address: addressB58 },
        );

        return {
            context: { slot: BigInt(response.context.slot) },
            value: response.value
                ? this.parseAccountV2(response.value)
                : null,
        };
    }

    async getCompressedAccountByHash(
        hash: Uint8Array,
    ): Promise<IndexerResponse<CompressedAccount | null>> {
        const hashB58 = this.bytesToBase58(hash);
        const response = await this.rpcCall<PhotonAccountV2 | null>(
            'getCompressedAccountByHashV2',
            { hash: hashB58 },
        );

        return {
            context: { slot: BigInt(response.context.slot) },
            value: response.value
                ? this.parseAccountV2(response.value)
                : null,
        };
    }

    async getCompressedTokenAccountsByOwner(
        owner: Address,
        options?: GetCompressedTokenAccountsOptions,
    ): Promise<IndexerResponse<ItemsWithCursor<CompressedTokenAccount>>> {
        const params: Record<string, unknown> = { owner: owner.toString() };
        if (options?.mint) {
            params.mint = options.mint.toString();
        }
        if (options?.cursor) {
            params.cursor = options.cursor;
        }
        if (options?.limit !== undefined) {
            params.limit = options.limit;
        }

        const response = await this.rpcCall<PhotonTokenAccountListV2>(
            'getCompressedTokenAccountsByOwnerV2',
            params,
        );

        return {
            context: { slot: BigInt(response.context.slot) },
            value: {
                items: response.value.items.map((item) =>
                    this.parseTokenAccountV2(item),
                ),
                cursor: response.value.cursor,
            },
        };
    }

    async getMultipleCompressedAccounts(
        addresses: Uint8Array[],
    ): Promise<IndexerResponse<(CompressedAccount | null)[]>> {
        const addressesB58 = addresses.map((a) => this.bytesToBase58(a));
        const response = await this.rpcCall<PhotonMultipleAccountsV2>(
            'getMultipleCompressedAccountsV2',
            { addresses: addressesB58 },
        );

        return {
            context: { slot: BigInt(response.context.slot) },
            value: response.value.items.map((item) =>
                item ? this.parseAccountV2(item) : null,
            ),
        };
    }

    async getValidityProof(
        hashes: Uint8Array[],
        newAddresses?: AddressWithTree[],
    ): Promise<IndexerResponse<ValidityProofWithContext>> {
        const hashesB58 = hashes.map((h) => this.bytesToBase58(h));
        const addressesParam = newAddresses?.map((a) => ({
            address: this.bytesToBase58(a.address),
            tree: a.tree.toString(),
        }));

        const response = await this.rpcCall<PhotonValidityProofV2>(
            'getValidityProofV2',
            {
                hashes: hashesB58,
                newAddressesWithTrees: addressesParam ?? [],
            },
        );

        return {
            context: { slot: BigInt(response.context.slot) },
            value: this.parseValidityProofV2(response.value),
        };
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    private async rpcCall<T>(
        method: string,
        params: unknown,
    ): Promise<{ context: { slot: number }; value: T }> {
        const request: JsonRpcRequest = {
            jsonrpc: '2.0',
            id: String(++this.requestId),
            method,
            params,
        };

        let response: Response;
        try {
            response = await fetch(this.endpoint, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(request),
            });
        } catch (e) {
            throw new IndexerError(
                IndexerErrorCode.NetworkError,
                `Failed to fetch from ${this.endpoint}: ${e}`,
                e,
            );
        }

        if (!response.ok) {
            throw new IndexerError(
                IndexerErrorCode.NetworkError,
                `HTTP error ${response.status}: ${response.statusText}`,
            );
        }

        let json: JsonRpcResponse<T>;
        try {
            // Parse JSON text manually to preserve big integer precision.
            // JSON.parse() silently truncates integers > 2^53.
            // Wrap large numbers as strings before parsing so BigInt()
            // conversion in parse methods receives the full value.
            const text = await response.text();
            const safeText = text.replace(
                /:\s*(\d{16,})\s*([,}\]])/g,
                ': "$1"$2',
            );
            json = JSON.parse(safeText) as JsonRpcResponse<T>;
        } catch (e) {
            throw new IndexerError(
                IndexerErrorCode.InvalidResponse,
                `Invalid JSON response: ${e}`,
                e,
            );
        }

        if (json.error) {
            throw new IndexerError(
                IndexerErrorCode.RpcError,
                `RPC error ${json.error.code}: ${json.error.message}`,
                json.error,
            );
        }

        if (!json.result) {
            throw new IndexerError(
                IndexerErrorCode.InvalidResponse,
                'Missing result in response',
            );
        }

        return json.result;
    }

    private parseTreeInfo(ctx: PhotonMerkleContextV2): TreeInfo {
        // Validate V2-only tree types
        assertV2Tree(ctx.treeType as TreeType);

        const info: TreeInfo = {
            tree: createAddress(ctx.tree),
            queue: createAddress(ctx.queue),
            treeType: ctx.treeType as TreeType,
        };
        if (ctx.cpiContext) {
            info.cpiContext = createAddress(ctx.cpiContext);
        }
        if (ctx.nextTreeContext) {
            info.nextTreeInfo = this.parseTreeInfo(ctx.nextTreeContext);
        }
        return info;
    }

    private parseAccountData(
        data: PhotonAccountData,
    ): CompressedAccountData {
        return {
            discriminator: this.bigintToBytes8(BigInt(data.discriminator)),
            data: this.base64Decode(data.data),
            dataHash: this.base58ToBytes(data.dataHash),
        };
    }

    private parseAccountV2(account: PhotonAccountV2): CompressedAccount {
        return {
            hash: this.base58ToBytes(account.hash),
            address: account.address
                ? this.base58ToBytes(account.address)
                : null,
            owner: createAddress(account.owner),
            lamports: BigInt(account.lamports),
            data: account.data ? this.parseAccountData(account.data) : null,
            leafIndex: account.leafIndex,
            treeInfo: this.parseTreeInfo(account.merkleContext),
            proveByIndex: Boolean(account.proveByIndex),
            seq: account.seq !== null ? BigInt(account.seq) : null,
            slotCreated: BigInt(account.slotCreated),
        };
    }

    private parseTokenData(data: PhotonTokenData): TokenData {
        return {
            mint: createAddress(data.mint),
            owner: createAddress(data.owner),
            amount: BigInt(data.amount),
            delegate: data.delegate ? createAddress(data.delegate) : null,
            state:
                data.state === 'frozen'
                    ? AccountState.Frozen
                    : AccountState.Initialized,
            tlv: data.tlv ? this.base64Decode(data.tlv) : null,
        };
    }

    private parseTokenAccountV2(
        tokenAccount: PhotonTokenAccountV2,
    ): CompressedTokenAccount {
        return {
            token: this.parseTokenData(tokenAccount.tokenData),
            account: this.parseAccountV2(tokenAccount.account),
        };
    }

    private parseRootIndex(ri: PhotonRootIndex): RootIndex {
        return {
            rootIndex: ri.rootIndex,
            proveByIndex: Boolean(ri.proveByIndex),
        };
    }

    private parseAccountProofInputs(
        input: PhotonAccountProofInputs,
    ): AccountProofInputs {
        return {
            hash: this.base58ToBytes(input.hash),
            root: this.base58ToBytes(input.root),
            rootIndex: this.parseRootIndex(input.rootIndex),
            leafIndex: input.leafIndex,
            treeInfo: this.parseTreeInfo(input.merkleContext),
        };
    }

    private parseAddressProofInputs(
        input: PhotonAddressProofInputs,
    ): AddressProofInputs {
        return {
            address: this.base58ToBytes(input.address),
            root: this.base58ToBytes(input.root),
            rootIndex: input.rootIndex,
            treeInfo: this.parseTreeInfo(input.merkleContext),
        };
    }

    private parseValidityProofV2(
        proof: PhotonValidityProofV2,
    ): ValidityProofWithContext {
        return {
            proof: proof.compressedProof
                ? {
                      a: Uint8Array.from(proof.compressedProof.a),
                      b: Uint8Array.from(proof.compressedProof.b),
                      c: Uint8Array.from(proof.compressedProof.c),
                  }
                : null,
            accounts: proof.accounts.map((a) => this.parseAccountProofInputs(a)),
            addresses: proof.addresses.map((a) =>
                this.parseAddressProofInputs(a),
            ),
        };
    }

    /**
     * Convert bytes to base58 string.
     * Uses the decoder because it decodes bytes FROM internal format TO base58 string.
     */
    private bytesToBase58(bytes: Uint8Array): string {
        return this.base58Decoder.decode(bytes);
    }

    /**
     * Convert base58 string to bytes.
     * Uses the encoder because it encodes base58 string TO internal byte format.
     */
    private base58ToBytes(str: string): Uint8Array {
        // The encoder returns ReadonlyUint8Array, so we need to copy to mutable Uint8Array
        return Uint8Array.from(this.base58Encoder.encode(str));
    }

    private base64Decode(str: string): Uint8Array {
        // Use atob for browser/node compatibility
        const binary = atob(str);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
        }
        return bytes;
    }

    private bigintToBytes8(value: bigint): Uint8Array {
        const bytes = new Uint8Array(8);
        let remaining = value;
        for (let i = 0; i < 8; i++) {
            bytes[i] = Number(remaining & 0xffn);
            remaining >>= 8n;
        }
        return bytes;
    }
}

// ============================================================================
// PHOTON API RESPONSE TYPES (Internal)
// ============================================================================

interface PhotonMerkleContextV2 {
    tree: string;
    queue: string;
    treeType: number;
    cpiContext?: string | null;
    nextTreeContext?: PhotonMerkleContextV2 | null;
}

interface PhotonAccountData {
    discriminator: string | number;
    data: string;
    dataHash: string;
}

interface PhotonAccountV2 {
    address: string | null;
    hash: string;
    data: PhotonAccountData | null;
    lamports: string | number;
    owner: string;
    leafIndex: number;
    seq: number | null;
    slotCreated: string | number;
    merkleContext: PhotonMerkleContextV2;
    proveByIndex: boolean | number;
}

interface PhotonTokenData {
    mint: string;
    owner: string;
    amount: string | number;
    delegate: string | null;
    state: string;
    tlv: string | null;
}

interface PhotonTokenAccountV2 {
    tokenData: PhotonTokenData;
    account: PhotonAccountV2;
}

interface PhotonTokenAccountListV2 {
    items: PhotonTokenAccountV2[];
    cursor: string | null;
}

interface PhotonMultipleAccountsV2 {
    items: (PhotonAccountV2 | null)[];
}

interface PhotonRootIndex {
    rootIndex: number;
    proveByIndex: boolean | number;
}

interface PhotonAccountProofInputs {
    hash: string;
    root: string;
    rootIndex: PhotonRootIndex;
    merkleContext: PhotonMerkleContextV2;
    leafIndex: number;
}

interface PhotonAddressProofInputs {
    address: string;
    root: string;
    rootIndex: number;
    merkleContext: PhotonMerkleContextV2;
}

interface PhotonCompressedProof {
    a: number[];
    b: number[];
    c: number[];
}

interface PhotonValidityProofV2 {
    compressedProof: PhotonCompressedProof | null;
    accounts: PhotonAccountProofInputs[];
    addresses: PhotonAddressProofInputs[];
}

// ============================================================================
// FACTORY FUNCTION
// ============================================================================

/**
 * Create a Light indexer client.
 *
 * @param endpoint - Photon API endpoint URL
 * @returns LightIndexer instance
 *
 * @example
 * ```typescript
 * const indexer = createLightIndexer('https://photon.helius.dev');
 * const accounts = await indexer.getCompressedTokenAccountsByOwner(owner);
 * const proof = await indexer.getValidityProof(hashes);
 * ```
 */
export function createLightIndexer(endpoint: string): LightIndexer {
    return new PhotonIndexer(endpoint);
}

/**
 * Check if Light indexer services are available.
 *
 * @param endpoint - Photon API endpoint URL
 * @returns True if the indexer is healthy
 */
export async function isLightIndexerAvailable(
    endpoint: string,
): Promise<boolean> {
    try {
        const response = await fetch(endpoint, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                jsonrpc: '2.0',
                id: '1',
                method: 'getIndexerHealth',
                params: {},
            }),
        });
        if (!response.ok) return false;
        const json = await response.json();
        return !json.error;
    } catch {
        return false;
    }
}
