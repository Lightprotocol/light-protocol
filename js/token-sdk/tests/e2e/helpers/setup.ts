/**
 * E2E test setup helpers.
 *
 * Wraps the legacy SDK (stateless.js, compressed-token) to provide
 * test fixtures: creating mints, funding accounts, and sending Kit v2
 * instructions through the web3.js v1 transaction pipeline.
 *
 * NOTE: No direct @solana/web3.js import — the PublicKey constructor is
 * extracted at runtime from objects returned by stateless.js.
 */

import {
    Rpc,
    createRpc,
    newAccountWithLamports,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
} from '@lightprotocol/stateless.js';
import { createMint, mintTo } from '@lightprotocol/compressed-token';

import { AccountRole, type Instruction } from '@solana/instructions';
import { type Address, address } from '@solana/addresses';

// ============================================================================
// LEGACY INTEROP — runtime-extracted from stateless.js's web3.js
// ============================================================================

/**
 * PublicKey constructor extracted lazily from the first funded account.
 * This avoids importing @solana/web3.js directly while ensuring we use
 * the exact same class the legacy SDK uses internally.
 */
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
// TYPE ALIASES (structural — no web3.js import)
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
    // Lazily extract the PublicKey constructor on first call.
    if (!PubKey) PubKey = signer.publicKey.constructor;
    return signer;
}

// ============================================================================
// MINT HELPERS
// ============================================================================

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

    // Pass publicKey (not Signer) — createMint adds any Signer-typed authority
    // to the transaction signers, but the authority is NOT a required signer.
    const { mint } = await createMint(
        rpc,
        payer as any,
        (mintAuthority as any).publicKey,
        decimals,
        undefined,
        undefined,
        undefined,
        freezeAuthority ? (freezeAuthority as any).publicKey : null,
    );
    return {
        mint,
        mintAuthority,
        mintAddress: toKitAddress(mint),
    };
}

export async function mintCompressedTokens(
    rpc: Rpc,
    payer: Signer,
    mint: any,
    to: any,
    authority: Signer,
    amount: number | bigint,
): Promise<string> {
    return mintTo(
        rpc,
        payer as any,
        mint,
        to,
        authority as any,
        Number(amount),
    );
}

// ============================================================================
// INSTRUCTION CONVERSION
// ============================================================================

/**
 * Convert a Kit v2 Instruction to a web3.js v1 TransactionInstruction-
 * compatible plain object (same shape: programId, keys, data).
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

// ============================================================================
// QUERY HELPERS
// ============================================================================

export async function getCompressedBalance(
    rpc: Rpc,
    owner: any,
    mint: any,
): Promise<bigint> {
    const accounts = await rpc.getCompressedTokenAccountsByOwner(owner, {
        mint,
    });
    return accounts.items.reduce(
        (sum: bigint, acc: any) => sum + BigInt(acc.parsed.amount.toString()),
        0n,
    );
}

export async function getCompressedAccountCount(
    rpc: Rpc,
    owner: any,
    mint: any,
): Promise<number> {
    const accounts = await rpc.getCompressedTokenAccountsByOwner(owner, {
        mint,
    });
    return accounts.items.length;
}

export type { Rpc };
