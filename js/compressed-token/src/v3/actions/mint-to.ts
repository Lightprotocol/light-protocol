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
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import { createMintToInstruction } from '../instructions/mint-to';
import { getMintInterface } from '../get-mint-interface';

export async function mintTo(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    recipientAccount: PublicKey,
    authority: Signer,
    amount: number | bigint,
    outputQueue?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const mintInfo = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        CTOKEN_PROGRAM_ID,
    );

    if (!mintInfo.merkleContext) {
        throw new Error('Mint does not have MerkleContext');
    }

    let outputStateTreeInfo: TreeInfo;
    if (!outputQueue) {
        const trees = await rpc.getStateTreeInfos();
        outputStateTreeInfo = selectStateTreeInfo(trees);
    } else {
        const trees = await rpc.getStateTreeInfos();
        outputStateTreeInfo = trees.find(
            t => t.queue.equals(outputQueue!) || t.tree.equals(outputQueue!),
        )!;
        if (!outputStateTreeInfo) {
            throw new Error('Could not find TreeInfo for provided outputQueue');
        }
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

    const ix = createMintToInstruction(
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
            mintSigner: mintInfo.mintContext!.mintSigner,
            bump: mintInfo.mintContext!.bump,
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
        outputStateTreeInfo,
        recipientAccount,
        amount,
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
