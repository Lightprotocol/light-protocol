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
import { findMintAddress } from '../../compressible';
import { createMint } from '../../actions/create-mint';

export { TokenMetadataInstructionData };

/**
 * Create and initialize a new mint (SPL, Token-2022, or Compressed Token).
 *
 * This is a unified interface that dispatches to either:
 * - SPL/Token-2022 mint creation when `programId` is TOKEN_PROGRAM_ID or TOKEN_2022_PROGRAM_ID
 * - Compressed token mint creation when `programId` is CTOKEN_PROGRAM_ID (default)
 *
 * @param rpc               RPC connection to use
 * @param payer             Fee payer
 * @param mintAuthority     Account that will control minting (must be Signer for compressed mints)
 * @param freezeAuthority   Optional: Account that will control freeze and thaw.
 * @param decimals          Location of the decimal place
 * @param keypair           Optional: Mint keypair. Defaults to a random keypair.
 * @param metadata          Optional: Token metadata (only used for compressed mints)
 * @param addressTreeInfo   Optional: Address tree info (only used for compressed mints)
 * @param outputStateTreeInfo Optional: Output state tree info (only used for compressed mints)
 * @param confirmOptions    Optional: Options for confirming the transaction
 * @param programId         Optional: Token program ID. Defaults to CTOKEN_PROGRAM_ID (compressed).
 *                          Set to TOKEN_PROGRAM_ID or TOKEN_2022_PROGRAM_ID for SPL mints.
 *
 * @return Object with mint address and transaction signature
 */
export async function createMintInterface(
    rpc: Rpc,
    payer: Signer,
    mintAuthority: PublicKey | Signer,
    freezeAuthority: PublicKey | Signer | null,
    decimals: number,
    keypair: Keypair = Keypair.generate(),
    metadata?: TokenMetadataInstructionData,
    addressTreeInfo?: AddressTreeInfo,
    outputStateTreeInfo?: TreeInfo,
    confirmOptions?: ConfirmOptions,
    programId: PublicKey = CTOKEN_PROGRAM_ID,
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
            freezeAuthority,
            decimals,
            keypair,
            confirmOptions,
            programId,
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
        metadata,
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
