/**
 * Light Protocol Token IDL
 *
 * Programmatic IDL definition for the Light Token program using Codama.
 * The program uses single-byte SPL-compatible discriminators (3-18) and
 * custom discriminators (100+) with Pinocchio-based instruction dispatch.
 */

import {
    rootNode,
    programNode,
    instructionNode,
    instructionAccountNode,
    instructionArgumentNode,
    pdaNode,
    pdaValueNode,
    pdaLinkNode,
    constantDiscriminatorNode,
    constantValueNode,
    constantPdaSeedNodeFromString,
    variablePdaSeedNode,
    numberTypeNode,
    numberValueNode,
    publicKeyTypeNode,
    publicKeyValueNode,
    booleanTypeNode,
    optionTypeNode,
    bytesTypeNode,
    structTypeNode,
    structFieldTypeNode,
    arrayTypeNode,
    fixedSizeTypeNode,
} from 'codama';

// ============================================================================
// CONSTANTS
// ============================================================================

export const LIGHT_TOKEN_PROGRAM_ID =
    'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m';
export const CPI_AUTHORITY = 'GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy';
export const MINT_ADDRESS_TREE = 'amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx';
export const SYSTEM_PROGRAM = '11111111111111111111111111111111';

// ============================================================================
// INSTRUCTION DISCRIMINATORS
// ============================================================================

/** SPL-compatible discriminators */
export const DISCRIMINATOR = {
    TRANSFER: 3,
    APPROVE: 4,
    REVOKE: 5,
    MINT_TO: 7,
    BURN: 8,
    CLOSE: 9,
    FREEZE: 10,
    THAW: 11,
    TRANSFER_CHECKED: 12,
    MINT_TO_CHECKED: 14,
    BURN_CHECKED: 15,
    CREATE_TOKEN_ACCOUNT: 18,
    CREATE_ATA: 100,
    TRANSFER2: 101,
    CREATE_ATA_IDEMPOTENT: 102,
    MINT_ACTION: 103,
} as const;

// ============================================================================
// TYPE DEFINITIONS
// ============================================================================

/** Compression mode enum for Transfer2 */
const compressionModeType = numberTypeNode('u8');

/** Compression struct for Transfer2 */
const compressionStructType = structTypeNode([
    structFieldTypeNode({ name: 'mode', type: numberTypeNode('u8') }),
    structFieldTypeNode({ name: 'amount', type: numberTypeNode('u64') }),
    structFieldTypeNode({ name: 'mint', type: numberTypeNode('u8') }),
    structFieldTypeNode({
        name: 'sourceOrRecipient',
        type: numberTypeNode('u8'),
    }),
    structFieldTypeNode({ name: 'authority', type: numberTypeNode('u8') }),
    structFieldTypeNode({
        name: 'poolAccountIndex',
        type: numberTypeNode('u8'),
    }),
    structFieldTypeNode({ name: 'poolIndex', type: numberTypeNode('u8') }),
    structFieldTypeNode({ name: 'bump', type: numberTypeNode('u8') }),
    structFieldTypeNode({ name: 'decimals', type: numberTypeNode('u8') }),
]);

/** Packed merkle context */
const packedMerkleContextType = structTypeNode([
    structFieldTypeNode({
        name: 'merkleTreePubkeyIndex',
        type: numberTypeNode('u8'),
    }),
    structFieldTypeNode({
        name: 'queuePubkeyIndex',
        type: numberTypeNode('u8'),
    }),
    structFieldTypeNode({ name: 'leafIndex', type: numberTypeNode('u32') }),
    structFieldTypeNode({ name: 'proveByIndex', type: booleanTypeNode() }),
]);

/** Input token data with context */
const multiInputTokenDataType = structTypeNode([
    structFieldTypeNode({ name: 'owner', type: numberTypeNode('u8') }),
    structFieldTypeNode({ name: 'amount', type: numberTypeNode('u64') }),
    structFieldTypeNode({ name: 'hasDelegate', type: booleanTypeNode() }),
    structFieldTypeNode({ name: 'delegate', type: numberTypeNode('u8') }),
    structFieldTypeNode({ name: 'mint', type: numberTypeNode('u8') }),
    structFieldTypeNode({ name: 'version', type: numberTypeNode('u8') }),
    structFieldTypeNode({
        name: 'merkleContext',
        type: packedMerkleContextType,
    }),
    structFieldTypeNode({ name: 'rootIndex', type: numberTypeNode('u16') }),
]);

/** Output token data */
const multiTokenOutputDataType = structTypeNode([
    structFieldTypeNode({ name: 'owner', type: numberTypeNode('u8') }),
    structFieldTypeNode({ name: 'amount', type: numberTypeNode('u64') }),
    structFieldTypeNode({ name: 'hasDelegate', type: booleanTypeNode() }),
    structFieldTypeNode({ name: 'delegate', type: numberTypeNode('u8') }),
    structFieldTypeNode({ name: 'mint', type: numberTypeNode('u8') }),
    structFieldTypeNode({ name: 'version', type: numberTypeNode('u8') }),
]);

/** CPI context */
const cpiContextType = structTypeNode([
    structFieldTypeNode({ name: 'setContext', type: booleanTypeNode() }),
    structFieldTypeNode({ name: 'firstSetContext', type: booleanTypeNode() }),
    structFieldTypeNode({
        name: 'cpiContextAccountIndex',
        type: numberTypeNode('u8'),
    }),
]);

/** Compressible extension instruction data */
const compressibleExtensionDataType = structTypeNode([
    structFieldTypeNode({
        name: 'tokenAccountVersion',
        type: numberTypeNode('u8'),
    }),
    structFieldTypeNode({ name: 'rentPayment', type: numberTypeNode('u16') }),
    structFieldTypeNode({
        name: 'compressionOnly',
        type: numberTypeNode('u8'),
    }),
    structFieldTypeNode({ name: 'writeTopUp', type: numberTypeNode('u32') }),
    structFieldTypeNode({
        name: 'compressToPubkey',
        type: optionTypeNode(
            structTypeNode([
                structFieldTypeNode({
                    name: 'bump',
                    type: numberTypeNode('u8'),
                }),
                structFieldTypeNode({
                    name: 'programId',
                    type: fixedSizeTypeNode(bytesTypeNode(), 32),
                }),
                structFieldTypeNode({
                    name: 'seeds',
                    type: arrayTypeNode(bytesTypeNode()),
                }),
            ]),
        ),
    }),
]);

// ============================================================================
// IDL ROOT
// ============================================================================

export const lightTokenIdl = rootNode(
    programNode({
        name: 'lightToken',
        publicKey: LIGHT_TOKEN_PROGRAM_ID,
        version: '1.0.0',
        docs: ['Light Protocol compressed token program'],

        // ========================================================================
        // PDAs
        // ========================================================================
        pdas: [
            pdaNode({
                name: 'associatedTokenAccount',
                seeds: [
                    variablePdaSeedNode('owner', publicKeyTypeNode()),
                    constantPdaSeedNodeFromString(
                        'utf8',
                        LIGHT_TOKEN_PROGRAM_ID,
                    ),
                    variablePdaSeedNode('mint', publicKeyTypeNode()),
                ],
                docs: [
                    'Associated token account PDA: [owner, LIGHT_TOKEN_PROGRAM_ID, mint]',
                ],
            }),
            pdaNode({
                name: 'lightMint',
                seeds: [
                    constantPdaSeedNodeFromString('utf8', 'compressed_mint'),
                    variablePdaSeedNode('mintSigner', publicKeyTypeNode()),
                ],
                docs: ['Light mint PDA: ["compressed_mint", mintSigner]'],
            }),
            pdaNode({
                name: 'splInterfacePool',
                seeds: [
                    constantPdaSeedNodeFromString('utf8', 'pool'),
                    variablePdaSeedNode('mint', publicKeyTypeNode()),
                ],
                docs: ['SPL interface pool PDA: ["pool", mint]'],
            }),
        ],

        // ========================================================================
        // ACCOUNTS (for generated types)
        // ========================================================================
        accounts: [],

        // ========================================================================
        // INSTRUCTIONS
        // ========================================================================
        instructions: [
            // ----------------------------------------------------------------------
            // CToken Transfer (discriminator: 3)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenTransfer',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.TRANSFER),
                        ),
                    ),
                ],
                docs: ['Transfer CToken between decompressed accounts'],
                accounts: [
                    instructionAccountNode({
                        name: 'source',
                        isSigner: false,
                        isWritable: true,
                        docs: ['Source CToken account'],
                    }),
                    instructionAccountNode({
                        name: 'destination',
                        isSigner: false,
                        isWritable: true,
                        docs: ['Destination CToken account'],
                    }),
                    instructionAccountNode({
                        name: 'authority',
                        isSigner: true,
                        isWritable: false,
                        docs: ['Authority (owner or delegate)'],
                    }),
                    instructionAccountNode({
                        name: 'systemProgram',
                        isSigner: false,
                        isWritable: false,
                        defaultValue: publicKeyValueNode(SYSTEM_PROGRAM),
                        docs: ['System program for rent top-up'],
                    }),
                    instructionAccountNode({
                        name: 'feePayer',
                        isSigner: true,
                        isWritable: true,
                        isOptional: true,
                        docs: ['Optional fee payer for rent top-ups'],
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(DISCRIMINATOR.TRANSFER),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'amount',
                        type: numberTypeNode('u64'),
                        docs: ['Amount to transfer'],
                    }),
                    instructionArgumentNode({
                        name: 'maxTopUp',
                        type: optionTypeNode(numberTypeNode('u16')),
                        docs: [
                            'Maximum lamports for rent top-up (0 = no limit)',
                        ],
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // CToken TransferChecked (discriminator: 12)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenTransferChecked',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.TRANSFER_CHECKED),
                        ),
                    ),
                ],
                docs: ['Transfer CToken with decimals validation'],
                accounts: [
                    instructionAccountNode({
                        name: 'source',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'mint',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'destination',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'authority',
                        isSigner: true,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'systemProgram',
                        isSigner: false,
                        isWritable: false,
                        defaultValue: publicKeyValueNode(SYSTEM_PROGRAM),
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(
                            DISCRIMINATOR.TRANSFER_CHECKED,
                        ),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'amount',
                        type: numberTypeNode('u64'),
                    }),
                    instructionArgumentNode({
                        name: 'decimals',
                        type: numberTypeNode('u8'),
                    }),
                    instructionArgumentNode({
                        name: 'maxTopUp',
                        type: optionTypeNode(numberTypeNode('u16')),
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // CToken Approve (discriminator: 4)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenApprove',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.APPROVE),
                        ),
                    ),
                ],
                docs: ['Approve delegate on decompressed CToken account'],
                accounts: [
                    instructionAccountNode({
                        name: 'tokenAccount',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'delegate',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'owner',
                        isSigner: true,
                        isWritable: false,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(DISCRIMINATOR.APPROVE),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'amount',
                        type: numberTypeNode('u64'),
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // CToken Revoke (discriminator: 5)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenRevoke',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.REVOKE),
                        ),
                    ),
                ],
                docs: ['Revoke delegate on decompressed CToken account'],
                accounts: [
                    instructionAccountNode({
                        name: 'tokenAccount',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'owner',
                        isSigner: true,
                        isWritable: false,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(DISCRIMINATOR.REVOKE),
                        defaultValueStrategy: 'omitted',
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // CToken MintTo (discriminator: 7)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenMintTo',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.MINT_TO),
                        ),
                    ),
                ],
                docs: ['Mint tokens to decompressed CToken account'],
                accounts: [
                    instructionAccountNode({
                        name: 'mint',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'tokenAccount',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'mintAuthority',
                        isSigner: true,
                        isWritable: false,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(DISCRIMINATOR.MINT_TO),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'amount',
                        type: numberTypeNode('u64'),
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // CToken MintToChecked (discriminator: 14)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenMintToChecked',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.MINT_TO_CHECKED),
                        ),
                    ),
                ],
                docs: ['Mint tokens with decimals validation'],
                accounts: [
                    instructionAccountNode({
                        name: 'mint',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'tokenAccount',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'mintAuthority',
                        isSigner: true,
                        isWritable: false,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(
                            DISCRIMINATOR.MINT_TO_CHECKED,
                        ),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'amount',
                        type: numberTypeNode('u64'),
                    }),
                    instructionArgumentNode({
                        name: 'decimals',
                        type: numberTypeNode('u8'),
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // CToken Burn (discriminator: 8)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenBurn',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.BURN),
                        ),
                    ),
                ],
                docs: ['Burn tokens from decompressed CToken account'],
                accounts: [
                    instructionAccountNode({
                        name: 'tokenAccount',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'mint',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'authority',
                        isSigner: true,
                        isWritable: false,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(DISCRIMINATOR.BURN),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'amount',
                        type: numberTypeNode('u64'),
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // CToken BurnChecked (discriminator: 15)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenBurnChecked',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.BURN_CHECKED),
                        ),
                    ),
                ],
                docs: ['Burn tokens with decimals validation'],
                accounts: [
                    instructionAccountNode({
                        name: 'tokenAccount',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'mint',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'authority',
                        isSigner: true,
                        isWritable: false,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(
                            DISCRIMINATOR.BURN_CHECKED,
                        ),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'amount',
                        type: numberTypeNode('u64'),
                    }),
                    instructionArgumentNode({
                        name: 'decimals',
                        type: numberTypeNode('u8'),
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // CToken Close (discriminator: 9)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenClose',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.CLOSE),
                        ),
                    ),
                ],
                docs: ['Close decompressed CToken account'],
                accounts: [
                    instructionAccountNode({
                        name: 'tokenAccount',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'destination',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'owner',
                        isSigner: true,
                        isWritable: false,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(DISCRIMINATOR.CLOSE),
                        defaultValueStrategy: 'omitted',
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // CToken Freeze (discriminator: 10)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenFreeze',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.FREEZE),
                        ),
                    ),
                ],
                docs: ['Freeze decompressed CToken account'],
                accounts: [
                    instructionAccountNode({
                        name: 'tokenAccount',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'mint',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'freezeAuthority',
                        isSigner: true,
                        isWritable: false,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(DISCRIMINATOR.FREEZE),
                        defaultValueStrategy: 'omitted',
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // CToken Thaw (discriminator: 11)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'ctokenThaw',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.THAW),
                        ),
                    ),
                ],
                docs: ['Thaw frozen decompressed CToken account'],
                accounts: [
                    instructionAccountNode({
                        name: 'tokenAccount',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'mint',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'freezeAuthority',
                        isSigner: true,
                        isWritable: false,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(DISCRIMINATOR.THAW),
                        defaultValueStrategy: 'omitted',
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // Create Token Account (discriminator: 18)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'createTokenAccount',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.CREATE_TOKEN_ACCOUNT),
                        ),
                    ),
                ],
                docs: [
                    'Create CToken account (equivalent to SPL InitializeAccount3)',
                ],
                accounts: [
                    instructionAccountNode({
                        name: 'owner',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'mint',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'payer',
                        isSigner: true,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'tokenAccount',
                        isSigner: false,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'systemProgram',
                        isSigner: false,
                        isWritable: false,
                        defaultValue: publicKeyValueNode(SYSTEM_PROGRAM),
                    }),
                    instructionAccountNode({
                        name: 'compressibleConfig',
                        isSigner: false,
                        isWritable: false,
                        isOptional: true,
                    }),
                    instructionAccountNode({
                        name: 'rentSponsor',
                        isSigner: false,
                        isWritable: true,
                        isOptional: true,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(
                            DISCRIMINATOR.CREATE_TOKEN_ACCOUNT,
                        ),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'compressibleConfig',
                        type: optionTypeNode(compressibleExtensionDataType),
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // Create Associated Token Account (discriminator: 100)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'createAssociatedTokenAccount',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.CREATE_ATA),
                        ),
                    ),
                ],
                docs: ['Create associated CToken account'],
                accounts: [
                    instructionAccountNode({
                        name: 'owner',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'mint',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'payer',
                        isSigner: true,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'associatedTokenAccount',
                        isSigner: false,
                        isWritable: true,
                        defaultValue: pdaValueNode(
                            pdaLinkNode('associatedTokenAccount'),
                        ),
                    }),
                    instructionAccountNode({
                        name: 'systemProgram',
                        isSigner: false,
                        isWritable: false,
                        defaultValue: publicKeyValueNode(SYSTEM_PROGRAM),
                    }),
                    instructionAccountNode({
                        name: 'compressibleConfig',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'rentSponsor',
                        isSigner: false,
                        isWritable: true,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(DISCRIMINATOR.CREATE_ATA),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'bump',
                        type: numberTypeNode('u8'),
                    }),
                    instructionArgumentNode({
                        name: 'compressibleConfig',
                        type: optionTypeNode(compressibleExtensionDataType),
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // Create Associated Token Account Idempotent (discriminator: 102)
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'createAssociatedTokenAccountIdempotent',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(
                                DISCRIMINATOR.CREATE_ATA_IDEMPOTENT,
                            ),
                        ),
                    ),
                ],
                docs: [
                    'Create associated CToken account (idempotent - no-op if exists)',
                ],
                accounts: [
                    instructionAccountNode({
                        name: 'owner',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'mint',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'payer',
                        isSigner: true,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'associatedTokenAccount',
                        isSigner: false,
                        isWritable: true,
                        defaultValue: pdaValueNode(
                            pdaLinkNode('associatedTokenAccount'),
                        ),
                    }),
                    instructionAccountNode({
                        name: 'systemProgram',
                        isSigner: false,
                        isWritable: false,
                        defaultValue: publicKeyValueNode(SYSTEM_PROGRAM),
                    }),
                    instructionAccountNode({
                        name: 'compressibleConfig',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'rentSponsor',
                        isSigner: false,
                        isWritable: true,
                    }),
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(
                            DISCRIMINATOR.CREATE_ATA_IDEMPOTENT,
                        ),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'bump',
                        type: numberTypeNode('u8'),
                    }),
                    instructionArgumentNode({
                        name: 'compressibleConfig',
                        type: optionTypeNode(compressibleExtensionDataType),
                    }),
                ],
            }),

            // ----------------------------------------------------------------------
            // Transfer2 (discriminator: 101) - Batch transfer instruction
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'transfer2',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.TRANSFER2),
                        ),
                    ),
                ],
                docs: [
                    'Batch transfer instruction for compressed/decompressed operations.',
                    'Supports: transfer, compress, decompress, compress-and-close.',
                ],
                accounts: [
                    instructionAccountNode({
                        name: 'feePayer',
                        isSigner: true,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'authority',
                        isSigner: true,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'lightSystemProgram',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'registeredProgramPda',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'accountCompressionAuthority',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'accountCompressionProgram',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'selfProgram',
                        isSigner: false,
                        isWritable: false,
                        defaultValue: publicKeyValueNode(
                            LIGHT_TOKEN_PROGRAM_ID,
                        ),
                    }),
                    instructionAccountNode({
                        name: 'systemProgram',
                        isSigner: false,
                        isWritable: false,
                        defaultValue: publicKeyValueNode(SYSTEM_PROGRAM),
                    }),
                    // Remaining accounts are dynamic based on the transfer
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(DISCRIMINATOR.TRANSFER2),
                        defaultValueStrategy: 'omitted',
                    }),
                    instructionArgumentNode({
                        name: 'withTransactionHash',
                        type: booleanTypeNode(),
                    }),
                    instructionArgumentNode({
                        name: 'withLamportsChangeAccountMerkleTreeIndex',
                        type: booleanTypeNode(),
                    }),
                    instructionArgumentNode({
                        name: 'lamportsChangeAccountMerkleTreeIndex',
                        type: numberTypeNode('u8'),
                    }),
                    instructionArgumentNode({
                        name: 'lamportsChangeAccountOwnerIndex',
                        type: numberTypeNode('u8'),
                    }),
                    instructionArgumentNode({
                        name: 'outputQueue',
                        type: numberTypeNode('u8'),
                    }),
                    instructionArgumentNode({
                        name: 'maxTopUp',
                        type: numberTypeNode('u16'),
                    }),
                    instructionArgumentNode({
                        name: 'cpiContext',
                        type: optionTypeNode(cpiContextType),
                    }),
                    instructionArgumentNode({
                        name: 'compressions',
                        type: optionTypeNode(
                            arrayTypeNode(compressionStructType),
                        ),
                    }),
                    // Note: proof, inTokenData, outTokenData, inLamports, outLamports, inTlv, outTlv
                    // are complex nested structures that will be handled by manual codecs
                ],
            }),

            // ----------------------------------------------------------------------
            // MintAction (discriminator: 103) - Batch mint operations
            // ----------------------------------------------------------------------
            instructionNode({
                name: 'mintAction',
                discriminators: [
                    constantDiscriminatorNode(
                        constantValueNode(
                            numberTypeNode('u8'),
                            numberValueNode(DISCRIMINATOR.MINT_ACTION),
                        ),
                    ),
                ],
                docs: [
                    'Batch instruction for compressed mint management.',
                    'Supports: CreateMint, MintTo, UpdateAuthorities, DecompressMint, etc.',
                ],
                accounts: [
                    instructionAccountNode({
                        name: 'feePayer',
                        isSigner: true,
                        isWritable: true,
                    }),
                    instructionAccountNode({
                        name: 'authority',
                        isSigner: true,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'lightSystemProgram',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'registeredProgramPda',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'accountCompressionAuthority',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'accountCompressionProgram',
                        isSigner: false,
                        isWritable: false,
                    }),
                    instructionAccountNode({
                        name: 'selfProgram',
                        isSigner: false,
                        isWritable: false,
                        defaultValue: publicKeyValueNode(
                            LIGHT_TOKEN_PROGRAM_ID,
                        ),
                    }),
                    instructionAccountNode({
                        name: 'systemProgram',
                        isSigner: false,
                        isWritable: false,
                        defaultValue: publicKeyValueNode(SYSTEM_PROGRAM),
                    }),
                    // Remaining accounts are dynamic based on the mint action
                ],
                arguments: [
                    instructionArgumentNode({
                        name: 'discriminator',
                        type: numberTypeNode('u8'),
                        defaultValue: numberValueNode(
                            DISCRIMINATOR.MINT_ACTION,
                        ),
                        defaultValueStrategy: 'omitted',
                    }),
                    // MintAction has complex nested data handled by manual codecs
                ],
            }),
        ],

        // ========================================================================
        // DEFINED TYPES
        // ========================================================================
        definedTypes: [],

        // ========================================================================
        // ERRORS
        // ========================================================================
        errors: [],
    }),
);

export default lightTokenIdl;
