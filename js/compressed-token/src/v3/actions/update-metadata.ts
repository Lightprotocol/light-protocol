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
    LIGHT_TOKEN_PROGRAM_ID,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import {
    createUpdateMetadataFieldInstruction,
    createUpdateMetadataAuthorityInstruction,
    createRemoveMetadataKeyInstruction,
} from '../instructions/update-metadata';
import { getMintInterface } from '../get-mint-interface';

/**
 * Update a metadata field on a compressed token mint.
 * Works for both compressed and decompressed mints.
 *
 * @param rpc            RPC connection
 * @param payer          Fee payer (signer)
 * @param mint           Mint address
 * @param authority      Metadata update authority (signer)
 * @param fieldType      Field to update: 'name', 'symbol', 'uri', or 'custom'
 * @param value          New value for the field
 * @param customKey      Custom key name (required if fieldType is 'custom')
 * @param extensionIndex Extension index (default: 0)
 * @param confirmOptions Optional confirm options
 */
export async function updateMetadataField(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    authority: Signer,
    fieldType: 'name' | 'symbol' | 'uri' | 'custom',
    value: string,
    customKey?: string,
    extensionIndex: number = 0,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const mintInterface = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        LIGHT_TOKEN_PROGRAM_ID,
    );

    if (!mintInterface.tokenMetadata || !mintInterface.merkleContext) {
        throw new Error('Mint does not have TokenMetadata extension');
    }

    // When mint is decompressed, no validity proof needed - program reads from CMint account
    const isDecompressed =
        mintInterface.mintContext?.cmintDecompressed ?? false;
    const validityProof = isDecompressed
        ? null
        : await rpc.getValidityProofV2(
              [
                  {
                      hash: bn(mintInterface.merkleContext.hash),
                      leafIndex: mintInterface.merkleContext.leafIndex,
                      treeInfo: mintInterface.merkleContext.treeInfo,
                      proveByIndex: mintInterface.merkleContext.proveByIndex,
                  },
              ],
              [],
              DerivationMode.compressible,
          );

    const ix = createUpdateMetadataFieldInstruction(
        mintInterface,
        authority.publicKey,
        payer.publicKey,
        validityProof,
        fieldType,
        value,
        customKey,
        extensionIndex,
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

/**
 * Update the metadata authority of a compressed token mint.
 * Works for both compressed and decompressed mints.
 *
 * @param rpc              RPC connection
 * @param payer            Fee payer (signer)
 * @param mint             Mint address
 * @param currentAuthority Current metadata update authority (signer)
 * @param newAuthority     New metadata update authority
 * @param extensionIndex   Extension index (default: 0)
 * @param confirmOptions   Optional confirm options
 */
export async function updateMetadataAuthority(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    currentAuthority: Signer,
    newAuthority: PublicKey,
    extensionIndex: number = 0,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const mintInterface = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        LIGHT_TOKEN_PROGRAM_ID,
    );

    if (!mintInterface.tokenMetadata || !mintInterface.merkleContext) {
        throw new Error('Mint does not have TokenMetadata extension');
    }

    // When mint is decompressed, no validity proof needed - program reads from CMint account
    const isDecompressed =
        mintInterface.mintContext?.cmintDecompressed ?? false;
    const validityProof = isDecompressed
        ? null
        : await rpc.getValidityProofV2(
              [
                  {
                      hash: bn(mintInterface.merkleContext.hash),
                      leafIndex: mintInterface.merkleContext.leafIndex,
                      treeInfo: mintInterface.merkleContext.treeInfo,
                      proveByIndex: mintInterface.merkleContext.proveByIndex,
                  },
              ],
              [],
              DerivationMode.compressible,
          );

    const ix = createUpdateMetadataAuthorityInstruction(
        mintInterface,
        currentAuthority.publicKey,
        newAuthority,
        payer.publicKey,
        validityProof,
        extensionIndex,
    );

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

/**
 * Remove a metadata key from a compressed token mint.
 * Works for both compressed and decompressed mints.
 *
 * @param rpc            RPC connection
 * @param payer          Fee payer (signer)
 * @param mint           Mint address
 * @param authority      Metadata update authority (signer)
 * @param key            Metadata key to remove
 * @param idempotent     If true, don't error if key doesn't exist (default: false)
 * @param extensionIndex Extension index (default: 0)
 * @param confirmOptions Optional confirm options
 */
export async function removeMetadataKey(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    authority: Signer,
    key: string,
    idempotent: boolean = false,
    extensionIndex: number = 0,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const mintInterface = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        LIGHT_TOKEN_PROGRAM_ID,
    );

    if (!mintInterface.tokenMetadata || !mintInterface.merkleContext) {
        throw new Error('Mint does not have TokenMetadata extension');
    }

    // When mint is decompressed, no validity proof needed - program reads from CMint account
    const isDecompressed =
        mintInterface.mintContext?.cmintDecompressed ?? false;
    const validityProof = isDecompressed
        ? null
        : await rpc.getValidityProofV2(
              [
                  {
                      hash: bn(mintInterface.merkleContext.hash),
                      leafIndex: mintInterface.merkleContext.leafIndex,
                      treeInfo: mintInterface.merkleContext.treeInfo,
                      proveByIndex: mintInterface.merkleContext.proveByIndex,
                  },
              ],
              [],
              DerivationMode.compressible,
          );

    const ix = createRemoveMetadataKeyInstruction(
        mintInterface,
        authority.publicKey,
        payer.publicKey,
        validityProof,
        key,
        idempotent,
        extensionIndex,
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
