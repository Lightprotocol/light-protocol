/**
 * Input for creating off-chain metadata JSON.
 * Compatible with Token-2022 and Metaplex standards.
 */
export interface OffChainTokenMetadata {
    /** Token name */
    name: string;
    /** Token symbol */
    symbol: string;
    /** Optional description */
    description?: string;
    /** Optional image URI */
    image?: string;
    /** Optional additional metadata key-value pairs */
    additionalMetadata?: Array<{ key: string; value: string }>;
}

/**
 * Off-chain JSON format for token metadata.
 * Standard format compatible with Token-2022 and Metaplex tooling.
 */
export interface OffChainTokenMetadataJson {
    name: string;
    symbol: string;
    description?: string;
    image?: string;
    additionalMetadata?: Array<{ key: string; value: string }>;
}

/**
 * Format metadata for off-chain storage.
 *
 * Returns a plain object ready to be uploaded using any storage provider
 * (umi uploader, custom IPFS/Arweave/S3 solution, etc.).
 *
 * @example
 * // With umi uploader
 * import { toOffChainMetadataJson } from '@lightprotocol/compressed-token';
 * import { irysUploader } from '@metaplex-foundation/umi-uploader-irys';
 *
 * const umi = createUmi(connection).use(irysUploader());
 * const metadataJson = toOffChainMetadataJson({
 *     name: 'My Token',
 *     symbol: 'MTK',
 *     description: 'A compressed token',
 *     image: 'https://example.com/image.png',
 * });
 * const uri = await umi.uploader.uploadJson(metadataJson);
 *
 * // Then use uri with createMint
 * await createMint(rpc, payer, { ...params, uri });
 */
export function toOffChainMetadataJson(
    meta: OffChainTokenMetadata,
): OffChainTokenMetadataJson {
    const json: OffChainTokenMetadataJson = {
        name: meta.name,
        symbol: meta.symbol,
    };

    if (meta.description !== undefined) {
        json.description = meta.description;
    }
    if (meta.image !== undefined) {
        json.image = meta.image;
    }
    if (
        meta.additionalMetadata !== undefined &&
        meta.additionalMetadata.length > 0
    ) {
        json.additionalMetadata = meta.additionalMetadata;
    }

    return json;
}
