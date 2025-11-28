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
} from '@lightprotocol/stateless.js';
import { createMintToCompressedInstruction } from '../instructions/mint-to-compressed';
import { getMintInterface } from '../helpers';

export async function mintToCompressed(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    authority: Signer,
    recipients: Array<{ recipient: PublicKey; amount: number | bigint }>,
    outputQueue?: PublicKey,
    tokensOutQueue?: PublicKey,
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

    if (!outputQueue) {
        const trees = await rpc.getStateTreeInfos();
        const tree = selectStateTreeInfo(trees);
        outputQueue = tree.queue;
    }

    if (!tokensOutQueue) {
        tokensOutQueue = outputQueue;
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

    const ix = createMintToCompressedInstruction({
        mintSigner: mint,
        authority: authority.publicKey,
        payer: payer.publicKey,
        validityProof,
        merkleContext: mintInfo.merkleContext,
        mintData: {
            supply: mintInfo.mint.supply,
            decimals: mintInfo.mint.decimals,
            mintAuthority: mintInfo.mint.mintAuthority,
            freezeAuthority: mintInfo.mint.freezeAuthority,
            splMint: mintInfo.mintContext!.splMint,
            splMintInitialized: mintInfo.mintContext!.splMintInitialized,
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
        outputQueue,
        tokensOutQueue,
        recipients,
        tokenAccountVersion,
    });

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
