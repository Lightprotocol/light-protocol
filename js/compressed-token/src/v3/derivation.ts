import {
    CTOKEN_PROGRAM_ID,
    deriveAddressV2,
    TreeInfo,
} from '@lightprotocol/stateless.js';
import { PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';

/**
 * Returns the light mint address as bytes.
 */
export function deriveCMintAddress(
    mintSeed: PublicKey,
    addressTreeInfo: TreeInfo,
) {
    // find_mint_address returns [light mint, bump], we want light mint
    // In JS, just use the mintSeed directly as the light mint address
    const address = deriveAddressV2(
        findMintAddress(mintSeed)[0].toBytes(),
        addressTreeInfo.tree,
        CTOKEN_PROGRAM_ID,
    );
    return Array.from(address.toBytes());
}

/// b"compressed_mint"
export const COMPRESSED_MINT_SEED: Buffer = Buffer.from([
    99, 111, 109, 112, 114, 101, 115, 115, 101, 100, 95, 109, 105, 110, 116,
]);

/**
 * Finds the SPL mint PDA for a light-token mint.
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
/// Uses light-token program ID.
export function getAssociatedCTokenAddressAndBump(
    owner: PublicKey,
    mint: PublicKey,
) {
    return PublicKey.findProgramAddressSync(
        [owner.toBuffer(), CTOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
        CTOKEN_PROGRAM_ID,
    );
}

/// Same as "getAssociatedTokenAddress" but with light-token program ID.
export function getAssociatedCTokenAddress(owner: PublicKey, mint: PublicKey) {
    return PublicKey.findProgramAddressSync(
        [owner.toBuffer(), CTOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
        CTOKEN_PROGRAM_ID,
    )[0];
}
