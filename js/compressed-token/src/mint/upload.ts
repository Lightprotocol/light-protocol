import { PublicKey } from '@solana/web3.js';
import { TokenMetadataInstructionData } from './instructions/create-mint';

/** Serialize our on-chain/client metadata. */
function buildMetadataJson(meta: TokenMetadataInstructionData): string {
    return JSON.stringify(
        {
            name: meta.name,
            symbol: meta.symbol,
            updateAuthority: meta.updateAuthority
                ? ((meta.updateAuthority as PublicKey).toBase58?.() ??
                  String(meta.updateAuthority))
                : null,
            additionalMetadata: meta.additionalMetadata ?? null,
            schema: 'light-ctoken-metadata@1',
        },
        null,
        2,
    );
}

/** Upload to AWS S3 using a pre-signed PUT URL. */
export async function uploadMetadataToAwsWithPresignedUrl(
    params: { presignedUrl: string; publicUrl: string },
    metadata: TokenMetadataInstructionData,
): Promise<TokenMetadataInstructionData> {
    const body = buildMetadataJson(metadata);
    const res = await fetch(params.presignedUrl, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body,
    });
    if (!res.ok)
        throw new Error(
            `aws s3 upload failed: ${res.status} ${res.statusText}`,
        );
    return { ...metadata, uri: params.publicUrl };
}

/**
 * Upload to AWS S3 using an S3Client instance.
 * Requires @aws-sdk/client-s3 as an optional peer dependency.
 *
 * @example
 * import { S3Client } from '@aws-sdk/client-s3';
 * const s3 = new S3Client({ region: 'us-east-1', credentials: {...} });
 * await uploadMetadataToAws(s3, { bucket: 'my-bucket', region: 'us-east-1' }, metadata);
 */
export async function uploadMetadataToAws(
    s3Client: any,
    params: { bucket: string; region: string; key?: string },
    metadata: TokenMetadataInstructionData,
): Promise<TokenMetadataInstructionData> {
    let PutObjectCommand: any;

    try {
        const awsSdk = await import('@aws-sdk/client-s3');
        PutObjectCommand = awsSdk.PutObjectCommand;
    } catch (error) {
        throw new Error(
            'AWS SDK not found. Install @aws-sdk/client-s3 to use uploadMetadataToAws: npm install @aws-sdk/client-s3',
        );
    }

    const key = params.key || `light-token-metadata/${Date.now()}.json`;
    const body = buildMetadataJson(metadata);

    const command = new PutObjectCommand({
        Bucket: params.bucket,
        Key: key,
        Body: body,
        ContentType: 'application/json',
    });

    await s3Client.send(command);

    const uri = `https://${params.bucket}.s3.${params.region}.amazonaws.com/${key}`;
    return { ...metadata, uri };
}

/** Upload to a generic IPFS node's add endpoint (multipart). */
export async function uploadMetadataToIpfs(
    params: { addEndpoint: string; authHeader?: string; gateway?: string },
    metadata: TokenMetadataInstructionData,
): Promise<TokenMetadataInstructionData> {
    const json = buildMetadataJson(metadata);
    const boundary =
        '--------------------------' + Math.random().toString(16).slice(2);
    const body =
        `--${boundary}\r\n` +
        `Content-Disposition: form-data; name="file"; filename="metadata.json"\r\n` +
        `Content-Type: application/json\r\n\r\n` +
        `${json}\r\n` +
        `--${boundary}--\r\n`;

    const headers: Record<string, string> = {
        'Content-Type': `multipart/form-data; boundary=${boundary}`,
    };
    if (params.authHeader) headers.Authorization = params.authHeader;

    const res = await fetch(params.addEndpoint, {
        method: 'POST',
        headers,
        body,
    });
    if (!res.ok)
        throw new Error(`ipfs upload failed: ${res.status} ${res.statusText}`);

    const text = await res.text();
    let cid = '' as string;
    try {
        const parsed = JSON.parse(text);
        cid = parsed?.Hash || parsed?.Cid || parsed?.cid || '';
    } catch (_) {
        const lines = text
            .split('\n')
            .map(l => l.trim())
            .filter(l => l.length > 0);
        for (let i = lines.length - 1; i >= 0 && !cid; i--) {
            try {
                const obj = JSON.parse(lines[i]);
                cid = obj?.Hash || obj?.Cid || obj?.cid || '';
            } catch (_) {
                // ignore
            }
        }
    }
    if (!cid) throw new Error('ipfs upload: missing CID in response');

    const gateway = (params.gateway || 'https://ipfs.io/ipfs').replace(
        /\/$/,
        '',
    );
    return { ...metadata, uri: `${gateway}/${cid}` };
}

/** Upload to Arweave via a provided HTTP endpoint (e.g., your Bundlr/Irys backend). */
export async function uploadMetadataToArweave(
    params: {
        endpoint: string;
        bearerToken?: string;
        headers?: Record<string, string>;
    },
    metadata: TokenMetadataInstructionData,
): Promise<TokenMetadataInstructionData> {
    const body = buildMetadataJson(metadata);
    const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        ...(params.headers || {}),
    };
    if (params.bearerToken)
        headers.Authorization = `Bearer ${params.bearerToken}`;

    const res = await fetch(params.endpoint, { method: 'POST', headers, body });
    if (!res.ok)
        throw new Error(
            `arweave upload failed: ${res.status} ${res.statusText}`,
        );
    const json = await res.json().catch(() => ({}) as any);
    const id: string | undefined = (json && (json.id as string)) || undefined;
    const uri: string | undefined = (json && (json.uri as string)) || undefined;
    if (uri) return { ...metadata, uri };
    if (id) return { ...metadata, uri: `https://arweave.net/${id}` };
    throw new Error('arweave upload: missing id/uri in response');
}

/** Upload to NFT.Storage using a Bearer API key. */
export async function uploadMetadataToNFTStorage(
    apiKey: string,
    metadata: TokenMetadataInstructionData,
): Promise<TokenMetadataInstructionData> {
    const body = buildMetadataJson(metadata);
    const res = await fetch('https://api.nft.storage/upload', {
        method: 'POST',
        headers: { Authorization: `Bearer ${apiKey}` },
        body,
    });
    if (!res.ok)
        throw new Error(
            `nft.storage upload failed: ${res.status} ${res.statusText}`,
        );
    const json = await res.json();
    const cid = json?.value?.cid ?? json?.cid;
    if (!cid) throw new Error('nft.storage: missing cid in response');
    return { ...metadata, uri: `https://nftstorage.link/ipfs/${cid}` };
}
