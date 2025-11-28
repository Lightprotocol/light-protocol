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
    createUpdateMetadataFieldInstruction,
    createUpdateMetadataAuthorityInstruction,
    createRemoveMetadataKeyInstruction,
} from '../instructions/update-metadata';
import { getMintInterface } from '../helpers';

export async function updateMetadataField(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    mintSigner: Signer,
    authority: Signer,
    fieldType: 'name' | 'symbol' | 'uri' | 'custom',
    value: string,
    customKey?: string,
    extensionIndex: number = 0,
    outputStateTreeInfo?: TreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());

    const mintInfo = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        CTOKEN_PROGRAM_ID,
    );

    if (!mintInfo.tokenMetadata || !mintInfo.merkleContext) {
        throw new Error('Mint does not have TokenMetadata extension');
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

    const ix = createUpdateMetadataFieldInstruction({
        mintSigner: mintSigner.publicKey,
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
            metadata: {
                updateAuthority: mintInfo.tokenMetadata.updateAuthority || null,
                name: mintInfo.tokenMetadata.name,
                symbol: mintInfo.tokenMetadata.symbol,
                uri: mintInfo.tokenMetadata.uri,
            },
        },
        outputQueue: outputStateTreeInfo.queue,
        fieldType,
        value,
        customKey,
        extensionIndex,
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

export async function updateMetadataAuthority(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    mintSigner: Signer,
    currentAuthority: Signer,
    newAuthority: PublicKey,
    extensionIndex: number = 0,
    outputStateTreeInfo?: TreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());

    const mintInfo = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        CTOKEN_PROGRAM_ID,
    );

    if (!mintInfo.tokenMetadata || !mintInfo.merkleContext) {
        throw new Error('Mint does not have TokenMetadata extension');
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

    const ix = createUpdateMetadataAuthorityInstruction({
        mintSigner: mintSigner.publicKey,
        currentAuthority: currentAuthority.publicKey,
        newAuthority,
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
            metadata: {
                updateAuthority: mintInfo.tokenMetadata.updateAuthority || null,
                name: mintInfo.tokenMetadata.name,
                symbol: mintInfo.tokenMetadata.symbol,
                uri: mintInfo.tokenMetadata.uri,
            },
        },
        outputQueue: outputStateTreeInfo.queue,
        extensionIndex,
    });

    const additionalSigners = currentAuthority.publicKey.equals(payer.publicKey)
        ? []
        : [currentAuthority];

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}

export async function removeMetadataKey(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    mintSigner: Signer,
    authority: Signer,
    key: string,
    idempotent: boolean = false,
    extensionIndex: number = 0,
    outputStateTreeInfo?: TreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());

    const mintInfo = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        CTOKEN_PROGRAM_ID,
    );

    if (!mintInfo.tokenMetadata || !mintInfo.merkleContext) {
        throw new Error('Mint does not have TokenMetadata extension');
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

    const ix = createRemoveMetadataKeyInstruction({
        mintSigner: mintSigner.publicKey,
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
            metadata: {
                updateAuthority: mintInfo.tokenMetadata.updateAuthority || null,
                name: mintInfo.tokenMetadata.name,
                symbol: mintInfo.tokenMetadata.symbol,
                uri: mintInfo.tokenMetadata.uri,
            },
        },
        outputQueue: outputStateTreeInfo.queue,
        key,
        idempotent,
        extensionIndex,
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
