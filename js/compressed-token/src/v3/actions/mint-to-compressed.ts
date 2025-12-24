import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    DerivationMode,
    bn,
    CTOKEN_PROGRAM_ID,
    selectStateTreeInfo,
    TreeInfo,
} from '@lightprotocol/stateless.js';
import { createMintToCompressedInstruction } from '../instructions/mint-to-compressed';
import { getMintInterface } from '../get-mint-interface';

/**
 * Mint compressed tokens directly to compressed accounts.
 *
 * @param rpc                   RPC connection
 * @param payer                 Fee payer
 * @param mint                  Mint address
 * @param authority             Mint authority (must sign)
 * @param recipients            Array of recipients with amounts
 * @param outputStateTreeInfo   Optional output state tree info (auto-fetched if not provided)
 * @param tokenAccountVersion   Token account version (default: 3)
 * @param confirmOptions        Optional confirm options
 */
export async function mintToCompressed(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    authority: Signer,
    recipients: Array<{ recipient: PublicKey; amount: number | bigint }>,
    outputStateTreeInfo?: TreeInfo,
    tokenAccountVersion: number = 3,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const mintInfo = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        CTOKEN_PROGRAM_ID,
    );

    if (!mintInfo.merkleContext) {
        throw new Error('Mint does not have MerkleContext');
    }

    // Auto-fetch output state tree info if not provided
    if (!outputStateTreeInfo) {
        const trees = await rpc.getStateTreeInfos();
        outputStateTreeInfo = selectStateTreeInfo(trees);
    }

    const validityProof = await rpc.getValidityProofV2(
        [
            {
                hash: bn(mintInfo.merkleContext.hash),
                leafIndex: mintInfo.merkleContext.leafIndex,
                treeInfo: mintInfo.merkleContext.treeInfo,
                proveByIndex: mintInfo.merkleContext.proveByIndex,
            },
        ],
        [],
        DerivationMode.compressible,
    );

    const ix = createMintToCompressedInstruction(
        authority.publicKey,
        payer.publicKey,
        validityProof,
        mintInfo.merkleContext,
        {
            supply: mintInfo.mint.supply,
            decimals: mintInfo.mint.decimals,
            mintAuthority: mintInfo.mint.mintAuthority,
            freezeAuthority: mintInfo.mint.freezeAuthority,
            splMint: mintInfo.mintContext!.splMint,
            cmintDecompressed: mintInfo.mintContext!.cmintDecompressed,
            version: mintInfo.mintContext!.version,
            metadata: mintInfo.tokenMetadata
                ? {
                      updateAuthority:
                          mintInfo.tokenMetadata.updateAuthority || null,
                      name: mintInfo.tokenMetadata.name,
                      symbol: mintInfo.tokenMetadata.symbol,
                      uri: mintInfo.tokenMetadata.uri,
                  }
                : undefined,
        },
        recipients,
        outputStateTreeInfo,
        tokenAccountVersion,
    );

    const additionalSigners = authority.publicKey.equals(payer.publicKey)
        ? []
        : [authority];

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}
