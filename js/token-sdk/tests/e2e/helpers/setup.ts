/**
 * E2E test setup helpers for CToken accounts.
 *
 * Uses the legacy SDK (stateless.js, compressed-token) to create CToken
 * fixtures: decompressed mints, on-chain CToken accounts with balances,
 * and a bridge to send Kit v2 instructions via web3.js v1 transactions.
 */

import {
    Rpc,
    createRpc,
    newAccountWithLamports,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
    VERSION,
    featureFlags,
} from '@lightprotocol/stateless.js';
import {
    createMintInterface,
    decompressMint,
    createAssociatedCTokenAccount,
    getAssociatedCTokenAddress,
    mintToCToken,
} from '@lightprotocol/compressed-token';

import { AccountRole, type Instruction } from '@solana/instructions';
import { type Address, address } from '@solana/addresses';

// Enable V2 + beta features for CToken operations
featureFlags.version = VERSION.V2;
featureFlags.enableBeta();

// ============================================================================
// LEGACY INTEROP — runtime-extracted from stateless.js's web3.js
// ============================================================================

let PubKey: any = null;

function pk(value: string): any {
    if (!PubKey) throw new Error('call fundAccount() before using pk()');
    return new PubKey(value);
}

// ============================================================================
// TEST RPC
// ============================================================================

const SOLANA_RPC = 'http://127.0.0.1:8899';
const COMPRESSION_RPC = 'http://127.0.0.1:8784';
const PROVER_RPC = 'http://127.0.0.1:3001';

export function getTestRpc(): Rpc {
    return createRpc(SOLANA_RPC, COMPRESSION_RPC, PROVER_RPC);
}

// ============================================================================
// VALIDATOR HEALTH CHECK
// ============================================================================

/**
 * Check if the local test validator is reachable.
 * Call this in beforeAll to skip tests when the validator is down.
 */
export async function ensureValidatorRunning(): Promise<void> {
    try {
        const response = await fetch(SOLANA_RPC, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                jsonrpc: '2.0',
                id: 1,
                method: 'getHealth',
            }),
            signal: AbortSignal.timeout(3000),
        });
        const json = (await response.json()) as { result?: string };
        if (json.result !== 'ok') {
            throw new Error(`Validator unhealthy: ${JSON.stringify(json)}`);
        }
    } catch {
        throw new Error(
            'Local test validator is not running. ' +
                'Start it with: ./cli/test_bin/run test-validator',
        );
    }
}

// ============================================================================
// TYPE ALIASES
// ============================================================================

/** web3.js v1 Signer shape (publicKey + secretKey). */
export type Signer = { publicKey: any; secretKey: Uint8Array };

// ============================================================================
// ACCOUNT HELPERS
// ============================================================================

export async function fundAccount(
    rpc: Rpc,
    lamports = 10e9,
): Promise<Signer> {
    const signer: any = await newAccountWithLamports(rpc, lamports);
    if (!PubKey) PubKey = signer.publicKey.constructor;
    return signer;
}

// ============================================================================
// CTOKEN MINT HELPERS
// ============================================================================

/**
 * Create a CToken mint: creates a compressed mint then decompresses it
 * so it exists as a CMint on-chain account.
 */
export async function createTestMint(
    rpc: Rpc,
    payer: Signer,
    decimals = 2,
    freezeAuthority?: Signer | null,
): Promise<{
    mint: any;
    mintAuthority: Signer;
    mintAddress: Address;
}> {
    const mintAuthority = await fundAccount(rpc, 1e9);

    // Step 1: Create compressed mint
    const result = await createMintInterface(
        rpc,
        payer as any,
        mintAuthority as any,
        freezeAuthority ? (freezeAuthority as any).publicKey : null,
        decimals,
    );
    const mint = result.mint;

    // Step 2: Decompress mint to create on-chain CMint account
    await decompressMint(rpc, payer as any, mint);

    return {
        mint,
        mintAuthority,
        mintAddress: toKitAddress(mint),
    };
}

// ============================================================================
// CTOKEN ACCOUNT HELPERS
// ============================================================================

/**
 * Create a CToken associated token account for the given owner.
 * Returns the on-chain CToken account address (web3.js PublicKey + Kit Address).
 */
export async function createCTokenAccount(
    rpc: Rpc,
    payer: Signer,
    owner: Signer,
    mint: any,
): Promise<{ ctokenPubkey: any; ctokenAddress: Address }> {
    await createAssociatedCTokenAccount(
        rpc,
        payer as any,
        (owner as any).publicKey,
        mint,
    );
    const ctokenPubkey = getAssociatedCTokenAddress(
        (owner as any).publicKey,
        mint,
    );
    return {
        ctokenPubkey,
        ctokenAddress: toKitAddress(ctokenPubkey),
    };
}

/**
 * Create a CToken account and mint tokens to it.
 */
export async function createCTokenWithBalance(
    rpc: Rpc,
    payer: Signer,
    mint: any,
    owner: Signer,
    mintAuthority: Signer,
    amount: number | bigint,
): Promise<{ ctokenPubkey: any; ctokenAddress: Address }> {
    const { ctokenPubkey, ctokenAddress } = await createCTokenAccount(
        rpc,
        payer,
        owner,
        mint,
    );

    // Mint tokens to the CToken account
    await mintToCToken(
        rpc,
        payer as any,
        mint,
        ctokenPubkey,
        mintAuthority as any,
        amount,
    );

    return { ctokenPubkey, ctokenAddress };
}

// ============================================================================
// CTOKEN STATE READERS
// ============================================================================

/**
 * Parsed CToken account info from on-chain data.
 * Follows SPL Token Account layout (first 165 bytes).
 */
export interface CTokenAccountData {
    mint: string;
    owner: string;
    amount: bigint;
    hasDelegate: boolean;
    delegate: string | null;
    /** 1 = initialized, 2 = frozen */
    state: number;
    delegatedAmount: bigint;
    hasCloseAuthority: boolean;
    closeAuthority: string | null;
}

function pubkeyToBase58(bytes: Uint8Array): string {
    // Use the PubKey constructor to convert bytes → base58
    return new PubKey(bytes).toBase58();
}

/**
 * Read and parse a CToken account from on-chain.
 */
export async function getCTokenAccountData(
    rpc: Rpc,
    ctokenPubkey: any,
): Promise<CTokenAccountData | null> {
    const info = await rpc.getAccountInfo(ctokenPubkey);
    if (!info || !info.data || info.data.length < 165) return null;

    const data = info.data;
    const view = new DataView(
        data.buffer,
        data.byteOffset,
        data.byteLength,
    );

    const mint = pubkeyToBase58(data.slice(0, 32));
    const owner = pubkeyToBase58(data.slice(32, 64));
    const amount = view.getBigUint64(64, true);

    const delegateOption = view.getUint32(72, true);
    const hasDelegate = delegateOption === 1;
    const delegate = hasDelegate
        ? pubkeyToBase58(data.slice(76, 108))
        : null;

    const state = data[108];

    const delegatedAmount = view.getBigUint64(121, true);

    const closeAuthorityOption = view.getUint32(129, true);
    const hasCloseAuthority = closeAuthorityOption === 1;
    const closeAuthority = hasCloseAuthority
        ? pubkeyToBase58(data.slice(133, 165))
        : null;

    return {
        mint,
        owner,
        amount,
        hasDelegate,
        delegate,
        state,
        delegatedAmount,
        hasCloseAuthority,
        closeAuthority,
    };
}

/**
 * Get the balance of a CToken account.
 */
export async function getCTokenBalance(
    rpc: Rpc,
    ctokenPubkey: any,
): Promise<bigint> {
    const data = await getCTokenAccountData(rpc, ctokenPubkey);
    if (!data) throw new Error('CToken account not found');
    return data.amount;
}

// ============================================================================
// INSTRUCTION CONVERSION
// ============================================================================

/**
 * Convert a Kit v2 Instruction to a web3.js v1 TransactionInstruction-
 * compatible plain object.
 */
export function toWeb3Instruction(ix: Instruction): any {
    return {
        programId: pk(ix.programAddress as string),
        keys: (ix.accounts ?? []).map((acc) => ({
            pubkey: pk(acc.address as string),
            isSigner:
                acc.role === AccountRole.READONLY_SIGNER ||
                acc.role === AccountRole.WRITABLE_SIGNER,
            isWritable:
                acc.role === AccountRole.WRITABLE ||
                acc.role === AccountRole.WRITABLE_SIGNER,
        })),
        data: Buffer.from(ix.data ?? new Uint8Array()),
    };
}

/** Convert a web3.js v1 PublicKey to a Kit v2 Address. */
export function toKitAddress(pubkey: any): Address {
    return address(pubkey.toBase58());
}

// ============================================================================
// TRANSACTION HELPERS
// ============================================================================

/** ComputeBudget SetComputeUnitLimit (variant 2, u32 LE units). */
function setComputeUnitLimit(units: number): any {
    const data = Buffer.alloc(5);
    data.writeUInt8(2, 0);
    data.writeUInt32LE(units, 1);
    return {
        programId: pk('ComputeBudget111111111111111111111111111111'),
        keys: [] as any[],
        data,
    };
}

export async function sendKitInstructions(
    rpc: Rpc,
    ixs: Instruction[],
    payer: Signer,
    signers: Signer[] = [],
): Promise<string> {
    const web3Ixs = [
        setComputeUnitLimit(1_000_000),
        ...ixs.map(toWeb3Instruction),
    ];

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer as any, signers as any[]);
    const tx = buildAndSignTx(
        web3Ixs as any[],
        payer as any,
        blockhash,
        additionalSigners,
    );
    return sendAndConfirmTx(rpc, tx);
}

export type { Rpc };
