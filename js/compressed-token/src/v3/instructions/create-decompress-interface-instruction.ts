import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import {
    LIGHT_TOKEN_PROGRAM_ID,
    LightSystemProgram,
    defaultStaticAccountsStruct,
    ParsedTokenAccount,
    ValidityProofWithContext,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../program';
import {
    encodeTransfer2InstructionData,
    Transfer2InstructionData,
    MultiInputTokenDataWithContext,
    COMPRESSION_MODE_DECOMPRESS,
    Compression,
} from '../layout/layout-transfer2';
import { MAX_TOP_UP, TokenDataVersion } from '../../constants';
import { SplInterfaceInfo } from '../../utils/get-token-pool-infos';

/**
 * Get token data version from compressed account discriminator.
 */
function getVersionFromDiscriminator(
    discriminator: number[] | undefined,
): number {
    if (!discriminator || discriminator.length < 8) {
        // Default to ShaFlat for new accounts without discriminator
        return TokenDataVersion.ShaFlat;
    }

    // V1 has discriminator[0] = 2
    if (discriminator[0] === 2) {
        return TokenDataVersion.V1;
    }

    // V2 and ShaFlat have version in discriminator[7]
    const versionByte = discriminator[7];
    if (versionByte === 3) {
        return TokenDataVersion.V2;
    }
    if (versionByte === 4) {
        return TokenDataVersion.ShaFlat;
    }

    // Default to ShaFlat
    return TokenDataVersion.ShaFlat;
}

/**
 * Build input token data for Transfer2 from parsed token accounts
 */
function buildInputTokenData(
    accounts: ParsedTokenAccount[],
    rootIndices: number[],
    packedAccountIndices: Map<string, number>,
): MultiInputTokenDataWithContext[] {
    return accounts.map((acc, i) => {
        const ownerKey = acc.parsed.owner.toBase58();
        const mintKey = acc.parsed.mint.toBase58();

        const version = getVersionFromDiscriminator(
            acc.compressedAccount.data?.discriminator,
        );

        return {
            owner: packedAccountIndices.get(ownerKey)!,
            amount: BigInt(acc.parsed.amount.toString()),
            hasDelegate: acc.parsed.delegate !== null,
            delegate: acc.parsed.delegate
                ? (packedAccountIndices.get(acc.parsed.delegate.toBase58()) ??
                  0)
                : 0,
            mint: packedAccountIndices.get(mintKey)!,
            version,
            merkleContext: {
                merkleTreePubkeyIndex: packedAccountIndices.get(
                    acc.compressedAccount.treeInfo.tree.toBase58(),
                )!,
                queuePubkeyIndex: packedAccountIndices.get(
                    acc.compressedAccount.treeInfo.queue.toBase58(),
                )!,
                leafIndex: acc.compressedAccount.leafIndex,
                proveByIndex: acc.compressedAccount.proveByIndex,
            },
            rootIndex: rootIndices[i],
        };
    });
}

/**
 * Create decompress instruction using Transfer2.
 *
 * @internal Use createLoadAtaInstructions instead.
 *
 * Supports decompressing to both c-token accounts and SPL token accounts:
 * - For c-token destinations: No splInterfaceInfo needed
 * - For SPL destinations: Provide splInterfaceInfo (token pool info) and decimals
 *
 * @param payer                        Fee payer public key
 * @param inputCompressedTokenAccounts Input compressed token accounts
 * @param toAddress                    Destination token account address (c-token or SPL ATA)
 * @param amount                       Amount to decompress
 * @param validityProof                Validity proof (contains compressedProof and rootIndices)
 * @param splInterfaceInfo             Optional: SPL interface info for SPL destinations
 * @param decimals                     Mint decimals (required for SPL destinations)
 * @param maxTopUp                     Optional cap on rent top-up (units of 1k lamports; default no cap)
 * @returns TransactionInstruction
 */
export function createDecompressInterfaceInstruction(
    payer: PublicKey,
    inputCompressedTokenAccounts: ParsedTokenAccount[],
    toAddress: PublicKey,
    amount: bigint,
    validityProof: ValidityProofWithContext,
    splInterfaceInfo: SplInterfaceInfo | undefined,
    decimals: number,
    maxTopUp?: number,
): TransactionInstruction {
    if (inputCompressedTokenAccounts.length === 0) {
        throw new Error('No input compressed token accounts provided');
    }

    const mint = inputCompressedTokenAccounts[0].parsed.mint;
    const owner = inputCompressedTokenAccounts[0].parsed.owner;

    // Build packed accounts map
    // Order: trees/queues first, then mint, owner, c-token account, c-token program
    const packedAccountIndices = new Map<string, number>();
    const packedAccounts: PublicKey[] = [];

    // Collect unique trees and queues
    const treeSet = new Set<string>();
    const queueSet = new Set<string>();
    for (const acc of inputCompressedTokenAccounts) {
        treeSet.add(acc.compressedAccount.treeInfo.tree.toBase58());
        queueSet.add(acc.compressedAccount.treeInfo.queue.toBase58());
    }

    // Add trees first (owned by account compression program)
    for (const tree of treeSet) {
        packedAccountIndices.set(tree, packedAccounts.length);
        packedAccounts.push(new PublicKey(tree));
    }

    let firstQueueIndex = 0;
    let isFirstQueue = true;
    for (const queue of queueSet) {
        if (isFirstQueue) {
            firstQueueIndex = packedAccounts.length;
            isFirstQueue = false;
        }
        packedAccountIndices.set(queue, packedAccounts.length);
        packedAccounts.push(new PublicKey(queue));
    }

    // Add mint
    const mintIndex = packedAccounts.length;
    packedAccountIndices.set(mint.toBase58(), mintIndex);
    packedAccounts.push(mint);

    // Add owner
    const ownerIndex = packedAccounts.length;
    packedAccountIndices.set(owner.toBase58(), ownerIndex);
    packedAccounts.push(owner);

    // Add destination token account (c-token or SPL)
    const destinationIndex = packedAccounts.length;
    packedAccountIndices.set(toAddress.toBase58(), destinationIndex);
    packedAccounts.push(toAddress);

    // Add unique delegate pubkeys from input accounts
    for (const acc of inputCompressedTokenAccounts) {
        if (acc.parsed.delegate) {
            const delegateKey = acc.parsed.delegate.toBase58();
            if (!packedAccountIndices.has(delegateKey)) {
                packedAccountIndices.set(delegateKey, packedAccounts.length);
                packedAccounts.push(acc.parsed.delegate);
            }
        }
    }

    // For SPL decompression, add pool account and token program
    let poolAccountIndex = 0;
    let poolIndex = 0;
    let poolBump = 0;
    let tokenProgramIndex = 0;

    if (splInterfaceInfo) {
        // Add SPL interface PDA (token pool)
        poolAccountIndex = packedAccounts.length;
        packedAccountIndices.set(
            splInterfaceInfo.splInterfacePda.toBase58(),
            poolAccountIndex,
        );
        packedAccounts.push(splInterfaceInfo.splInterfacePda);

        // Add SPL token program
        tokenProgramIndex = packedAccounts.length;
        packedAccountIndices.set(
            splInterfaceInfo.tokenProgram.toBase58(),
            tokenProgramIndex,
        );
        packedAccounts.push(splInterfaceInfo.tokenProgram);

        poolIndex = splInterfaceInfo.poolIndex;
        poolBump = splInterfaceInfo.bump;
    }

    // Build input token data
    const inTokenData = buildInputTokenData(
        inputCompressedTokenAccounts,
        validityProof.rootIndices,
        packedAccountIndices,
    );

    // Calculate total input amount and change
    const totalInputAmount = inputCompressedTokenAccounts.reduce(
        (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
        BigInt(0),
    );
    const changeAmount = totalInputAmount - amount;

    const outTokenData: {
        owner: number;
        amount: bigint;
        hasDelegate: boolean;
        delegate: number;
        mint: number;
        version: number;
    }[] = [];

    if (changeAmount > 0) {
        const version = getVersionFromDiscriminator(
            inputCompressedTokenAccounts[0].compressedAccount.data
                ?.discriminator,
        );

        outTokenData.push({
            owner: ownerIndex,
            amount: changeAmount,
            hasDelegate: false,
            delegate: 0,
            mint: mintIndex,
            version,
        });
    }

    // Build decompress compression
    // For c-token: pool values are 0 (unused)
    // For SPL: pool values point to SPL interface PDA
    const compressions: Compression[] = [
        {
            mode: COMPRESSION_MODE_DECOMPRESS,
            amount,
            mint: mintIndex,
            sourceOrRecipient: destinationIndex,
            authority: 0, // Not needed for decompress
            poolAccountIndex: splInterfaceInfo ? poolAccountIndex : 0,
            poolIndex: splInterfaceInfo ? poolIndex : 0,
            bump: splInterfaceInfo ? poolBump : 0,
            decimals,
        },
    ];

    // Build Transfer2 instruction data
    const instructionData: Transfer2InstructionData = {
        withTransactionHash: false,
        withLamportsChangeAccountMerkleTreeIndex: false,
        lamportsChangeAccountMerkleTreeIndex: 0,
        lamportsChangeAccountOwnerIndex: 0,
        outputQueue: firstQueueIndex, // First queue in packed accounts
        maxTopUp: maxTopUp ?? MAX_TOP_UP,
        cpiContext: null,
        compressions,
        proof: validityProof.compressedProof
            ? {
                  a: Array.from(validityProof.compressedProof.a),
                  b: Array.from(validityProof.compressedProof.b),
                  c: Array.from(validityProof.compressedProof.c),
              }
            : null,
        inTokenData,
        outTokenData,
        inLamports: null,
        outLamports: null,
        inTlv: null,
        outTlv: null,
    };

    const data = encodeTransfer2InstructionData(instructionData);

    // Build accounts for Transfer2 with compressed accounts (full path)
    const {
        accountCompressionAuthority,
        registeredProgramPda,
        accountCompressionProgram,
    } = defaultStaticAccountsStruct();

    const keys = [
        // 0: light_system_program (non-mutable)
        {
            pubkey: LightSystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        // 1: fee_payer (signer, mutable)
        { pubkey: payer, isSigner: true, isWritable: true },
        // 2: cpi_authority_pda
        {
            pubkey: CompressedTokenProgram.deriveCpiAuthorityPda,
            isSigner: false,
            isWritable: false,
        },
        // 3: registered_program_pda
        {
            pubkey: registeredProgramPda,
            isSigner: false,
            isWritable: false,
        },
        // 4: account_compression_authority
        {
            pubkey: accountCompressionAuthority,
            isSigner: false,
            isWritable: false,
        },
        // 5: account_compression_program
        {
            pubkey: accountCompressionProgram,
            isSigner: false,
            isWritable: false,
        },
        // 6: system_program
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        // 7+: packed_accounts (trees/queues come first)
        ...packedAccounts.map((pubkey, i) => {
            // Trees need to be writable
            const isTreeOrQueue = i < treeSet.size + queueSet.size;
            // Destination account needs to be writable
            const isDestination = pubkey.equals(toAddress);
            // SPL interface PDA (pool) needs to be writable for SPL decompression
            const isPool =
                splInterfaceInfo !== undefined &&
                pubkey.equals(splInterfaceInfo.splInterfacePda);
            // Owner must be marked as signer in packed accounts
            const isOwner = i === ownerIndex;
            return {
                pubkey,
                isSigner: isOwner,
                isWritable: isTreeOrQueue || isDestination || isPool,
            };
        }),
    ];

    return new TransactionInstruction({
        programId: CompressedTokenProgram.programId,
        keys,
        data,
    });
}
