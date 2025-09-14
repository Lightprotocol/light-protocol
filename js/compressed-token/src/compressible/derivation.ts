import {
    CTOKEN_PROGRAM_ID,
    deriveAddressV2,
    TreeInfo,
} from '@lightprotocol/stateless.js';
import { PublicKey } from '@solana/web3.js';

/**
 * Returns the compressed mint address as a Array (32 bytes).
 */
export function deriveCompressedMintAddress(
    mintSeed: PublicKey,
    addressTreeInfo: TreeInfo,
) {
    // find_spl_mint_address returns [splMint, bump], we want splMint
    // In JS, just use the mintSeed directly as the SPL mint address
    const address = deriveAddressV2(
        findMintAddress(mintSeed)[0].toBytes(),
        addressTreeInfo.tree.toBytes(),
        CTOKEN_PROGRAM_ID.toBytes(),
    );
    return Array.from(address);
}

/// b"compressed_mint"
export const COMPRESSED_MINT_SEED = Buffer.from([
    99, 111, 109, 112, 114, 101, 115, 115, 101, 100, 95, 109, 105, 110, 116,
]);

/**
 * Finds the SPL mint PDA for a compressed mint.
 * @param mintSeed The mint seed public key.
 * @returns [PDA, bump]
 */
export function findMintAddress(mintSigner: PublicKey): [PublicKey, number] {
    const [address, bump] = PublicKey.findProgramAddressSync(
        [COMPRESSED_MINT_SEED, mintSigner.toBuffer()],
        CTOKEN_PROGRAM_ID,
    );
    return [address, bump];
}

/// Same as "getAssociatedTokenAddress" but returns the bump as well.
/// Uses compressed token program ID.
export function getAssociatedCTokenAddressAndBump(
    owner: PublicKey,
    mint: PublicKey,
) {
    return PublicKey.findProgramAddressSync(
        [owner.toBuffer(), CTOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
        CTOKEN_PROGRAM_ID,
    );
}

/// Same as "getAssociatedTokenAddress" but implicitly uses compressed token program ID.
export function getAssociatedCTokenAddress(owner: PublicKey, mint: PublicKey) {
    return PublicKey.findProgramAddressSync(
        [owner.toBuffer(), CTOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
        CTOKEN_PROGRAM_ID,
    )[0];
}
