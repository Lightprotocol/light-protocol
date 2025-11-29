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
    TreeInfo,
    selectStateTreeInfo,
    DerivationMode,
    bn,
    CTOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import {
    createUpdateMintAuthorityInstruction,
    createUpdateFreezeAuthorityInstruction,
} from '../instructions/update-mint';
import { getMintInterface } from '../helpers';

export async function updateMintAuthority(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    mintSigner: Signer,
    currentMintAuthority: Signer,
    newMintAuthority: PublicKey | null,
    outputStateTreeInfo?: TreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());

    const mintInfo = await getMintInterface(
        rpc,
        mint,
        undefined,
        CTOKEN_PROGRAM_ID,
    );

    if (!mintInfo.merkleContext) {
        throw new Error('Mint does not have MerkleContext');
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

    const ix = createUpdateMintAuthorityInstruction(
        currentMintAuthority.publicKey,
        newMintAuthority,
        payer.publicKey,
        validityProof,
        mintInfo.merkleContext,
        {
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
        outputStateTreeInfo.queue,
    );

    const additionalSigners = currentMintAuthority.publicKey.equals(
        payer.publicKey,
    )
        ? []
        : [currentMintAuthority];

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}

export async function updateFreezeAuthority(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    mintSigner: Signer,
    currentFreezeAuthority: Signer,
    newFreezeAuthority: PublicKey | null,
    outputStateTreeInfo?: TreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());

    const mintInfo = await getMintInterface(
        rpc,
        mint,
        undefined,
        CTOKEN_PROGRAM_ID,
    );
    if (!mintInfo.merkleContext) {
        throw new Error('Mint does not have MerkleContext');
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

    const ix = createUpdateFreezeAuthorityInstruction(
        currentFreezeAuthority.publicKey,
        newFreezeAuthority,
        payer.publicKey,
        validityProof,
        mintInfo.merkleContext,
        {
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
        outputStateTreeInfo.queue,
    );

    const additionalSigners = currentFreezeAuthority.publicKey.equals(
        payer.publicKey,
    )
        ? []
        : [currentFreezeAuthority];

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}
