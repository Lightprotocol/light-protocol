import {
    ComputeBudgetProgram,
    ConfirmOptions,
    Keypair,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    dedupeSigner,
    sendAndConfirmTx,
    TreeInfo,
    AddressTreeInfo,
    selectStateTreeInfo,
    getBatchAddressTreeInfo,
    DerivationMode,
    CTOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import { TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import {
    createMintInstruction,
    TokenMetadataInstructionData,
} from '../instructions/create-mint';
import { findMintAddress } from '../derivation';
import { createMint } from '../../actions/create-mint';

export { TokenMetadataInstructionData };

/**
 * Create and initialize a new mint for SPL/T22/c-token.
 *
 * @param rpc                   RPC connection to use
 * @param payer                 Fee payer
 * @param mintAuthority         Account that will control minting (signer for c-token mints)
 * @param freezeAuthority       Account that will control freeze and thaw (optional)
 * @param decimals              Location of the decimal place
 * @param keypair               Mint keypair (defaults to a random keypair)
 * @param confirmOptions        Confirm options
 * @param programId             Token program ID (defaults to CTOKEN_PROGRAM_ID)
 * @param tokenMetadata         Optional token metadata (c-token mints only)
 * @param outputStateTreeInfo   Optional output state tree info (c-token mints only)
 * @param addressTreeInfo       Optional address tree info (c-token mints only)
 *
 * @returns Object with mint address and transaction signature
 */
export async function createMintInterface(
    rpc: Rpc,
    payer: Signer,
    mintAuthority: PublicKey | Signer,
    freezeAuthority: PublicKey | Signer | null,
    decimals: number,
    keypair: Keypair = Keypair.generate(),
    confirmOptions?: ConfirmOptions,
    programId: PublicKey = CTOKEN_PROGRAM_ID,
    tokenMetadata?: TokenMetadataInstructionData,
    outputStateTreeInfo?: TreeInfo,
    addressTreeInfo?: AddressTreeInfo,
): Promise<{ mint: PublicKey; transactionSignature: TransactionSignature }> {
    // Dispatch to SPL/Token-2022 mint creation
    if (
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        return createMint(
            rpc,
            payer,
            mintAuthority,
            decimals,
            keypair,
            confirmOptions,
            programId,
            freezeAuthority,
        );
    }

    // Default: compressed token mint creation
    if (!('secretKey' in mintAuthority)) {
        throw new Error(
            'mintAuthority must be a Signer for compressed token mints',
        );
    }

    const resolvedFreezeAuthority =
        freezeAuthority && 'secretKey' in freezeAuthority
            ? freezeAuthority.publicKey
            : (freezeAuthority as PublicKey | null);

    addressTreeInfo = addressTreeInfo ?? getBatchAddressTreeInfo();
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());

    const validityProof = await rpc.getValidityProofV2(
        [],
        [
            {
                address: findMintAddress(keypair.publicKey)[0].toBytes(),
                treeInfo: addressTreeInfo,
            },
        ],
        DerivationMode.compressible,
    );

    const ix = createMintInstruction(
        keypair.publicKey,
        decimals,
        mintAuthority.publicKey,
        resolvedFreezeAuthority,
        payer.publicKey,
        validityProof,
        addressTreeInfo,
        outputStateTreeInfo,
        tokenMetadata,
    );

    const additionalSigners = dedupeSigner(payer, [keypair, mintAuthority]);
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );
    const txId = await sendAndConfirmTx(rpc, tx, {
        ...confirmOptions,
        skipPreflight: true,
    });

    const mint = findMintAddress(keypair.publicKey);
    return { mint: mint[0], transactionSignature: txId };
}
