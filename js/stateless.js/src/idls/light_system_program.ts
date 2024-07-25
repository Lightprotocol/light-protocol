export type LightSystemProgram = {
    version: '0.5.0';
    name: 'light_system_program';
    constants: [
        {
            name: 'SOL_POOL_PDA_SEED';
            type: 'bytes';
            value: '[115, 111, 108, 95, 112, 111, 111, 108, 95, 112, 100, 97]';
        },
    ];
    instructions: [
        {
            name: 'initCpiContextAccount';
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'cpiContextAccount';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'associatedMerkleTree';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [];
        },
        {
            name: 'invoke';
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: [
                        'Fee payer needs to be mutable to pay rollover and protocol fees.',
                    ];
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'noopProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionAuthority';
                    isMut: false;
                    isSigner: false;
                    docs: [
                        'This pda is used to invoke the account compression program.',
                    ];
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                    docs: ['Merkle trees.'];
                },
                {
                    name: 'solPoolPda';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                    docs: [
                        'Sol pool pda is used to store the native sol that has been compressed.',
                        "It's only required when compressing or decompressing sol.",
                    ];
                },
                {
                    name: 'decompressionRecipient';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                    docs: [
                        'Only needs to be provided for decompression as a recipient for the',
                        'decompressed sol.',
                        'Compressed sol originate from authority.',
                    ];
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'inputs';
                    type: 'bytes';
                },
            ];
        },
        {
            name: 'invokeCpi';
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: [
                        'Fee payer needs to be mutable to pay rollover and protocol fees.',
                    ];
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'noopProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionAuthority';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'invokingProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'solPoolPda';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'decompressionRecipient';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'cpiContextAccount';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                },
            ];
            args: [
                {
                    name: 'inputs';
                    type: 'bytes';
                },
            ];
        },
        {
            name: 'stubIdlBuild';
            docs: [
                'This function is a stub to allow Anchor to include the input types in',
                'the IDL. It should not be included in production builds nor be called in',
                'practice.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: [
                        'Fee payer needs to be mutable to pay rollover and protocol fees.',
                    ];
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'noopProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionAuthority';
                    isMut: false;
                    isSigner: false;
                    docs: [
                        'This pda is used to invoke the account compression program.',
                    ];
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                    docs: ['Merkle trees.'];
                },
                {
                    name: 'solPoolPda';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                    docs: [
                        'Sol pool pda is used to store the native sol that has been compressed.',
                        "It's only required when compressing or decompressing sol.",
                    ];
                },
                {
                    name: 'decompressionRecipient';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                    docs: [
                        'Only needs to be provided for decompression as a recipient for the',
                        'decompressed sol.',
                        'Compressed sol originate from authority.',
                    ];
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'inputs1';
                    type: {
                        defined: 'InstructionDataInvoke';
                    };
                },
                {
                    name: 'inputs2';
                    type: {
                        defined: 'InstructionDataInvokeCpi';
                    };
                },
                {
                    name: 'inputs3';
                    type: {
                        defined: 'PublicTransactionEvent';
                    };
                },
            ];
        },
    ];
    accounts: [
        {
            name: 'stateMerkleTreeAccount';
            docs: [
                'Concurrent state Merkle tree used for public compressed transactions.',
            ];
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'metadata';
                        type: {
                            defined: 'MerkleTreeMetadata';
                        };
                    },
                ];
            };
        },
        {
            name: 'cpiContextAccount';
            docs: [
                'Collects instruction data without executing a compressed transaction.',
                'Signer checks are performed on instruction data.',
                'Collected instruction data is combined with the instruction data of the executing cpi,',
                'and executed as a single transaction.',
                'This enables to use input compressed accounts that are owned by multiple programs,',
                'with one zero-knowledge proof.',
            ];
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'feePayer';
                        type: 'publicKey';
                    },
                    {
                        name: 'associatedMerkleTree';
                        type: 'publicKey';
                    },
                    {
                        name: 'context';
                        type: {
                            vec: {
                                defined: 'InstructionDataInvokeCpi';
                            };
                        };
                    },
                ];
            };
        },
    ];
    types: [
        {
            name: 'AccessMetadata';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'owner';
                        docs: ['Owner of the Merkle tree.'];
                        type: 'publicKey';
                    },
                    {
                        name: 'programOwner';
                        docs: [
                            'Program owner of the Merkle tree. This will be used for program owned Merkle trees.',
                        ];
                        type: 'publicKey';
                    },
                ];
            };
        },
        {
            name: 'MerkleTreeMetadata';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'accessMetadata';
                        type: {
                            defined: 'AccessMetadata';
                        };
                    },
                    {
                        name: 'rolloverMetadata';
                        type: {
                            defined: 'RolloverMetadata';
                        };
                    },
                    {
                        name: 'associatedQueue';
                        type: 'publicKey';
                    },
                    {
                        name: 'nextMerkleTree';
                        type: 'publicKey';
                    },
                ];
            };
        },
        {
            name: 'RolloverMetadata';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'index';
                        docs: ['Unique index.'];
                        type: 'u64';
                    },
                    {
                        name: 'rolloverFee';
                        docs: [
                            'This fee is used for rent for the next account.',
                            'It accumulates in the account so that once the corresponding Merkle tree account is full it can be rolled over',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'rolloverThreshold';
                        docs: [
                            'The threshold in percentage points when the account should be rolled over (95 corresponds to 95% filled).',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'networkFee';
                        docs: ['Tip for maintaining the account.'];
                        type: 'u64';
                    },
                    {
                        name: 'rolledoverSlot';
                        docs: [
                            'The slot when the account was rolled over, a rolled over account should not be written to.',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'closeThreshold';
                        docs: [
                            'If current slot is greater than rolledover_slot + close_threshold and',
                            "the account is empty it can be closed. No 'close' functionality has been",
                            'implemented yet.',
                        ];
                        type: 'u64';
                    },
                ];
            };
        },
        {
            name: 'InstructionDataInvoke';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'proof';
                        type: {
                            option: {
                                defined: 'CompressedProof';
                            };
                        };
                    },
                    {
                        name: 'inputCompressedAccountsWithMerkleContext';
                        type: {
                            vec: {
                                defined: 'PackedCompressedAccountWithMerkleContext';
                            };
                        };
                    },
                    {
                        name: 'outputCompressedAccounts';
                        type: {
                            vec: {
                                defined: 'OutputCompressedAccountWithPackedContext';
                            };
                        };
                    },
                    {
                        name: 'relayFee';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'newAddressParams';
                        type: {
                            vec: {
                                defined: 'NewAddressParamsPacked';
                            };
                        };
                    },
                    {
                        name: 'compressOrDecompressLamports';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'isCompress';
                        type: 'bool';
                    },
                ];
            };
        },
        {
            name: 'NewAddressParamsPacked';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'seed';
                        type: {
                            array: ['u8', 32];
                        };
                    },
                    {
                        name: 'addressQueueAccountIndex';
                        type: 'u8';
                    },
                    {
                        name: 'addressMerkleTreeAccountIndex';
                        type: 'u8';
                    },
                    {
                        name: 'addressMerkleTreeRootIndex';
                        type: 'u16';
                    },
                ];
            };
        },
        {
            name: 'OutputCompressedAccountWithPackedContext';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'compressedAccount';
                        type: {
                            defined: 'CompressedAccount';
                        };
                    },
                    {
                        name: 'merkleTreeIndex';
                        type: 'u8';
                    },
                ];
            };
        },
        {
            name: 'CompressedProof';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'a';
                        type: {
                            array: ['u8', 32];
                        };
                    },
                    {
                        name: 'b';
                        type: {
                            array: ['u8', 64];
                        };
                    },
                    {
                        name: 'c';
                        type: {
                            array: ['u8', 32];
                        };
                    },
                ];
            };
        },
        {
            name: 'InstructionDataInvokeCpi';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'proof';
                        type: {
                            option: {
                                defined: 'CompressedProof';
                            };
                        };
                    },
                    {
                        name: 'newAddressParams';
                        type: {
                            vec: {
                                defined: 'NewAddressParamsPacked';
                            };
                        };
                    },
                    {
                        name: 'inputCompressedAccountsWithMerkleContext';
                        type: {
                            vec: {
                                defined: 'PackedCompressedAccountWithMerkleContext';
                            };
                        };
                    },
                    {
                        name: 'outputCompressedAccounts';
                        type: {
                            vec: {
                                defined: 'OutputCompressedAccountWithPackedContext';
                            };
                        };
                    },
                    {
                        name: 'relayFee';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'compressOrDecompressLamports';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'isCompress';
                        type: 'bool';
                    },
                    {
                        name: 'signerSeeds';
                        type: {
                            vec: 'bytes';
                        };
                    },
                    {
                        name: 'cpiContext';
                        type: {
                            option: {
                                defined: 'CompressedCpiContext';
                            };
                        };
                    },
                ];
            };
        },
        {
            name: 'CompressedCpiContext';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'setContext';
                        docs: [
                            'Is set by the program that is invoking the CPI to signal that is should',
                            'set the cpi context.',
                        ];
                        type: 'bool';
                    },
                    {
                        name: 'firstSetContext';
                        docs: [
                            'Is set to wipe the cpi context since someone could have set it before',
                            'with unrelated data.',
                        ];
                        type: 'bool';
                    },
                    {
                        name: 'cpiContextAccountIndex';
                        docs: [
                            'Index of cpi context account in remaining accounts.',
                        ];
                        type: 'u8';
                    },
                ];
            };
        },
        {
            name: 'CompressedAccount';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'owner';
                        type: 'publicKey';
                    },
                    {
                        name: 'lamports';
                        type: 'u64';
                    },
                    {
                        name: 'address';
                        type: {
                            option: {
                                array: ['u8', 32];
                            };
                        };
                    },
                    {
                        name: 'data';
                        type: {
                            option: {
                                defined: 'CompressedAccountData';
                            };
                        };
                    },
                ];
            };
        },
        {
            name: 'CompressedAccountData';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'discriminator';
                        type: {
                            array: ['u8', 8];
                        };
                    },
                    {
                        name: 'data';
                        type: 'bytes';
                    },
                    {
                        name: 'dataHash';
                        type: {
                            array: ['u8', 32];
                        };
                    },
                ];
            };
        },
        {
            name: 'PackedCompressedAccountWithMerkleContext';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'compressedAccount';
                        type: {
                            defined: 'CompressedAccount';
                        };
                    },
                    {
                        name: 'merkleContext';
                        type: {
                            defined: 'PackedMerkleContext';
                        };
                    },
                    {
                        name: 'rootIndex';
                        docs: [
                            'Index of root used in inclusion validity proof.',
                        ];
                        type: 'u16';
                    },
                ];
            };
        },
        {
            name: 'PackedMerkleContext';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'merkleTreePubkeyIndex';
                        type: 'u8';
                    },
                    {
                        name: 'nullifierQueuePubkeyIndex';
                        type: 'u8';
                    },
                    {
                        name: 'leafIndex';
                        type: 'u32';
                    },
                    {
                        name: 'queueIndex';
                        docs: [
                            'Index of leaf in queue. Placeholder of batched Merkle tree updates',
                            'currently unimplemented.',
                        ];
                        type: {
                            option: {
                                defined: 'QueueIndex';
                            };
                        };
                    },
                ];
            };
        },
        {
            name: 'QueueIndex';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'queueId';
                        docs: ['Id of queue in queue account.'];
                        type: 'u8';
                    },
                    {
                        name: 'index';
                        docs: ['Index of compressed account hash in queue.'];
                        type: 'u16';
                    },
                ];
            };
        },
        {
            name: 'MerkleTreeSequenceNumber';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'pubkey';
                        type: 'publicKey';
                    },
                    {
                        name: 'seq';
                        type: 'u64';
                    },
                ];
            };
        },
        {
            name: 'PublicTransactionEvent';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'inputCompressedAccountHashes';
                        type: {
                            vec: {
                                array: ['u8', 32];
                            };
                        };
                    },
                    {
                        name: 'outputCompressedAccountHashes';
                        type: {
                            vec: {
                                array: ['u8', 32];
                            };
                        };
                    },
                    {
                        name: 'outputCompressedAccounts';
                        type: {
                            vec: {
                                defined: 'OutputCompressedAccountWithPackedContext';
                            };
                        };
                    },
                    {
                        name: 'outputLeafIndices';
                        type: {
                            vec: 'u32';
                        };
                    },
                    {
                        name: 'sequenceNumbers';
                        type: {
                            vec: {
                                defined: 'MerkleTreeSequenceNumber';
                            };
                        };
                    },
                    {
                        name: 'relayFee';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'isCompress';
                        type: 'bool';
                    },
                    {
                        name: 'compressOrDecompressLamports';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'pubkeyArray';
                        type: {
                            vec: 'publicKey';
                        };
                    },
                    {
                        name: 'message';
                        type: {
                            option: 'bytes';
                        };
                    },
                ];
            };
        },
    ];
    errors: [
        {
            code: 6000;
            name: 'SumCheckFailed';
            msg: 'Sum check failed';
        },
        {
            code: 6001;
            name: 'SignerCheckFailed';
            msg: 'Signer check failed';
        },
        {
            code: 6002;
            name: 'CpiSignerCheckFailed';
            msg: 'Cpi signer check failed';
        },
        {
            code: 6003;
            name: 'ComputeInputSumFailed';
            msg: 'Computing input sum failed.';
        },
        {
            code: 6004;
            name: 'ComputeOutputSumFailed';
            msg: 'Computing output sum failed.';
        },
        {
            code: 6005;
            name: 'ComputeRpcSumFailed';
            msg: 'Computing rpc sum failed.';
        },
        {
            code: 6006;
            name: 'InvalidNoopPubkey';
            msg: 'InvalidNoopPubkey';
        },
        {
            code: 6007;
            name: 'InvalidAddress';
            msg: 'InvalidAddress';
        },
        {
            code: 6008;
            name: 'DeriveAddressError';
            msg: 'DeriveAddressError';
        },
        {
            code: 6009;
            name: 'CompressedSolPdaUndefinedForCompressSol';
            msg: 'CompressedSolPdaUndefinedForCompressSol';
        },
        {
            code: 6010;
            name: 'DeCompressLamportsUndefinedForCompressSol';
            msg: 'DeCompressLamportsUndefinedForCompressSol';
        },
        {
            code: 6011;
            name: 'CompressedSolPdaUndefinedForDecompressSol';
            msg: 'CompressedSolPdaUndefinedForDecompressSol';
        },
        {
            code: 6012;
            name: 'DeCompressLamportsUndefinedForDecompressSol';
            msg: 'DeCompressLamportsUndefinedForDecompressSol';
        },
        {
            code: 6013;
            name: 'DecompressRecipientUndefinedForDecompressSol';
            msg: 'DecompressRecipientUndefinedForDecompressSol';
        },
        {
            code: 6014;
            name: 'WriteAccessCheckFailed';
            msg: 'WriteAccessCheckFailed';
        },
        {
            code: 6015;
            name: 'InvokingProgramNotProvided';
            msg: 'InvokingProgramNotProvided';
        },
        {
            code: 6016;
            name: 'InvalidCapacity';
            msg: 'InvalidCapacity';
        },
        {
            code: 6017;
            name: 'InvalidMerkleTreeOwner';
            msg: 'InvalidMerkleTreeOwner';
        },
        {
            code: 6018;
            name: 'ProofIsNone';
            msg: 'ProofIsNone';
        },
        {
            code: 6019;
            name: 'ProofIsSome';
            msg: 'Proof is some but no input compressed accounts or new addresses provided.';
        },
        {
            code: 6020;
            name: 'EmptyInputs';
            msg: 'EmptyInputs';
        },
        {
            code: 6021;
            name: 'CpiContextAccountUndefined';
            msg: 'CpiContextAccountUndefined';
        },
        {
            code: 6022;
            name: 'CpiContextEmpty';
            msg: 'CpiContextEmpty';
        },
        {
            code: 6023;
            name: 'CpiContextMissing';
            msg: 'CpiContextMissing';
        },
        {
            code: 6024;
            name: 'DecompressionRecipientDefined';
            msg: 'DecompressionRecipientDefined';
        },
        {
            code: 6025;
            name: 'SolPoolPdaDefined';
            msg: 'SolPoolPdaDefined';
        },
        {
            code: 6026;
            name: 'AppendStateFailed';
            msg: 'AppendStateFailed';
        },
        {
            code: 6027;
            name: 'InstructionNotCallable';
            msg: 'The instruction is not callable';
        },
        {
            code: 6028;
            name: 'CpiContextFeePayerMismatch';
            msg: 'CpiContextFeePayerMismatch';
        },
        {
            code: 6029;
            name: 'CpiContextAssociatedMerkleTreeMismatch';
            msg: 'CpiContextAssociatedMerkleTreeMismatch';
        },
        {
            code: 6030;
            name: 'NoInputs';
            msg: 'NoInputs';
        },
        {
            code: 6031;
            name: 'InputMerkleTreeIndicesNotInOrder';
            msg: 'Input merkle tree indices are not in ascending order.';
        },
        {
            code: 6032;
            name: 'OutputMerkleTreeIndicesNotInOrder';
            msg: 'Output merkle tree indices are not in ascending order.';
        },
    ];
};

export const IDL: LightSystemProgram = {
    version: '0.5.0',
    name: 'light_system_program',
    constants: [
        {
            name: 'SOL_POOL_PDA_SEED',
            type: 'bytes',
            value: '[115, 111, 108, 95, 112, 111, 111, 108, 95, 112, 100, 97]',
        },
    ],
    instructions: [
        {
            name: 'initCpiContextAccount',
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'cpiContextAccount',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'associatedMerkleTree',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [],
        },
        {
            name: 'invoke',
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: [
                        'Fee payer needs to be mutable to pay rollover and protocol fees.',
                    ],
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'noopProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionAuthority',
                    isMut: false,
                    isSigner: false,
                    docs: [
                        'This pda is used to invoke the account compression program.',
                    ],
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                    docs: ['Merkle trees.'],
                },
                {
                    name: 'solPoolPda',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                    docs: [
                        'Sol pool pda is used to store the native sol that has been compressed.',
                        "It's only required when compressing or decompressing sol.",
                    ],
                },
                {
                    name: 'decompressionRecipient',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                    docs: [
                        'Only needs to be provided for decompression as a recipient for the',
                        'decompressed sol.',
                        'Compressed sol originate from authority.',
                    ],
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'inputs',
                    type: 'bytes',
                },
            ],
        },
        {
            name: 'invokeCpi',
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: [
                        'Fee payer needs to be mutable to pay rollover and protocol fees.',
                    ],
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'noopProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionAuthority',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'invokingProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'solPoolPda',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'decompressionRecipient',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'cpiContextAccount',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                },
            ],
            args: [
                {
                    name: 'inputs',
                    type: 'bytes',
                },
            ],
        },
        {
            name: 'stubIdlBuild',
            docs: [
                'This function is a stub to allow Anchor to include the input types in',
                'the IDL. It should not be included in production builds nor be called in',
                'practice.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: [
                        'Fee payer needs to be mutable to pay rollover and protocol fees.',
                    ],
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'noopProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionAuthority',
                    isMut: false,
                    isSigner: false,
                    docs: [
                        'This pda is used to invoke the account compression program.',
                    ],
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                    docs: ['Merkle trees.'],
                },
                {
                    name: 'solPoolPda',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                    docs: [
                        'Sol pool pda is used to store the native sol that has been compressed.',
                        "It's only required when compressing or decompressing sol.",
                    ],
                },
                {
                    name: 'decompressionRecipient',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                    docs: [
                        'Only needs to be provided for decompression as a recipient for the',
                        'decompressed sol.',
                        'Compressed sol originate from authority.',
                    ],
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'inputs1',
                    type: {
                        defined: 'InstructionDataInvoke',
                    },
                },
                {
                    name: 'inputs2',
                    type: {
                        defined: 'InstructionDataInvokeCpi',
                    },
                },
                {
                    name: 'inputs3',
                    type: {
                        defined: 'PublicTransactionEvent',
                    },
                },
            ],
        },
    ],
    accounts: [
        {
            name: 'stateMerkleTreeAccount',
            docs: [
                'Concurrent state Merkle tree used for public compressed transactions.',
            ],
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'metadata',
                        type: {
                            defined: 'MerkleTreeMetadata',
                        },
                    },
                ],
            },
        },
        {
            name: 'cpiContextAccount',
            docs: [
                'Collects instruction data without executing a compressed transaction.',
                'Signer checks are performed on instruction data.',
                'Collected instruction data is combined with the instruction data of the executing cpi,',
                'and executed as a single transaction.',
                'This enables to use input compressed accounts that are owned by multiple programs,',
                'with one zero-knowledge proof.',
            ],
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'feePayer',
                        type: 'publicKey',
                    },
                    {
                        name: 'associatedMerkleTree',
                        type: 'publicKey',
                    },
                    {
                        name: 'context',
                        type: {
                            vec: {
                                defined: 'InstructionDataInvokeCpi',
                            },
                        },
                    },
                ],
            },
        },
    ],
    types: [
        {
            name: 'AccessMetadata',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'owner',
                        docs: ['Owner of the Merkle tree.'],
                        type: 'publicKey',
                    },
                    {
                        name: 'programOwner',
                        docs: [
                            'Program owner of the Merkle tree. This will be used for program owned Merkle trees.',
                        ],
                        type: 'publicKey',
                    },
                ],
            },
        },
        {
            name: 'MerkleTreeMetadata',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'accessMetadata',
                        type: {
                            defined: 'AccessMetadata',
                        },
                    },
                    {
                        name: 'rolloverMetadata',
                        type: {
                            defined: 'RolloverMetadata',
                        },
                    },
                    {
                        name: 'associatedQueue',
                        type: 'publicKey',
                    },
                    {
                        name: 'nextMerkleTree',
                        type: 'publicKey',
                    },
                ],
            },
        },
        {
            name: 'RolloverMetadata',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'index',
                        docs: ['Unique index.'],
                        type: 'u64',
                    },
                    {
                        name: 'rolloverFee',
                        docs: [
                            'This fee is used for rent for the next account.',
                            'It accumulates in the account so that once the corresponding Merkle tree account is full it can be rolled over',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'rolloverThreshold',
                        docs: [
                            'The threshold in percentage points when the account should be rolled over (95 corresponds to 95% filled).',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'networkFee',
                        docs: ['Tip for maintaining the account.'],
                        type: 'u64',
                    },
                    {
                        name: 'rolledoverSlot',
                        docs: [
                            'The slot when the account was rolled over, a rolled over account should not be written to.',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'closeThreshold',
                        docs: [
                            'If current slot is greater than rolledover_slot + close_threshold and',
                            "the account is empty it can be closed. No 'close' functionality has been",
                            'implemented yet.',
                        ],
                        type: 'u64',
                    },
                ],
            },
        },
        {
            name: 'InstructionDataInvoke',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'proof',
                        type: {
                            option: {
                                defined: 'CompressedProof',
                            },
                        },
                    },
                    {
                        name: 'inputCompressedAccountsWithMerkleContext',
                        type: {
                            vec: {
                                defined:
                                    'PackedCompressedAccountWithMerkleContext',
                            },
                        },
                    },
                    {
                        name: 'outputCompressedAccounts',
                        type: {
                            vec: {
                                defined:
                                    'OutputCompressedAccountWithPackedContext',
                            },
                        },
                    },
                    {
                        name: 'relayFee',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'newAddressParams',
                        type: {
                            vec: {
                                defined: 'NewAddressParamsPacked',
                            },
                        },
                    },
                    {
                        name: 'compressOrDecompressLamports',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'isCompress',
                        type: 'bool',
                    },
                ],
            },
        },
        {
            name: 'NewAddressParamsPacked',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'seed',
                        type: {
                            array: ['u8', 32],
                        },
                    },
                    {
                        name: 'addressQueueAccountIndex',
                        type: 'u8',
                    },
                    {
                        name: 'addressMerkleTreeAccountIndex',
                        type: 'u8',
                    },
                    {
                        name: 'addressMerkleTreeRootIndex',
                        type: 'u16',
                    },
                ],
            },
        },
        {
            name: 'OutputCompressedAccountWithPackedContext',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'compressedAccount',
                        type: {
                            defined: 'CompressedAccount',
                        },
                    },
                    {
                        name: 'merkleTreeIndex',
                        type: 'u8',
                    },
                ],
            },
        },
        {
            name: 'CompressedProof',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'a',
                        type: {
                            array: ['u8', 32],
                        },
                    },
                    {
                        name: 'b',
                        type: {
                            array: ['u8', 64],
                        },
                    },
                    {
                        name: 'c',
                        type: {
                            array: ['u8', 32],
                        },
                    },
                ],
            },
        },
        {
            name: 'InstructionDataInvokeCpi',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'proof',
                        type: {
                            option: {
                                defined: 'CompressedProof',
                            },
                        },
                    },
                    {
                        name: 'newAddressParams',
                        type: {
                            vec: {
                                defined: 'NewAddressParamsPacked',
                            },
                        },
                    },
                    {
                        name: 'inputCompressedAccountsWithMerkleContext',
                        type: {
                            vec: {
                                defined:
                                    'PackedCompressedAccountWithMerkleContext',
                            },
                        },
                    },
                    {
                        name: 'outputCompressedAccounts',
                        type: {
                            vec: {
                                defined:
                                    'OutputCompressedAccountWithPackedContext',
                            },
                        },
                    },
                    {
                        name: 'relayFee',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'compressOrDecompressLamports',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'isCompress',
                        type: 'bool',
                    },
                    {
                        name: 'signerSeeds',
                        type: {
                            vec: 'bytes',
                        },
                    },
                    {
                        name: 'cpiContext',
                        type: {
                            option: {
                                defined: 'CompressedCpiContext',
                            },
                        },
                    },
                ],
            },
        },
        {
            name: 'CompressedCpiContext',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'setContext',
                        docs: [
                            'Is set by the program that is invoking the CPI to signal that is should',
                            'set the cpi context.',
                        ],
                        type: 'bool',
                    },
                    {
                        name: 'firstSetContext',
                        docs: [
                            'Is set to wipe the cpi context since someone could have set it before',
                            'with unrelated data.',
                        ],
                        type: 'bool',
                    },
                    {
                        name: 'cpiContextAccountIndex',
                        docs: [
                            'Index of cpi context account in remaining accounts.',
                        ],
                        type: 'u8',
                    },
                ],
            },
        },
        {
            name: 'CompressedAccount',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'owner',
                        type: 'publicKey',
                    },
                    {
                        name: 'lamports',
                        type: 'u64',
                    },
                    {
                        name: 'address',
                        type: {
                            option: {
                                array: ['u8', 32],
                            },
                        },
                    },
                    {
                        name: 'data',
                        type: {
                            option: {
                                defined: 'CompressedAccountData',
                            },
                        },
                    },
                ],
            },
        },
        {
            name: 'CompressedAccountData',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'discriminator',
                        type: {
                            array: ['u8', 8],
                        },
                    },
                    {
                        name: 'data',
                        type: 'bytes',
                    },
                    {
                        name: 'dataHash',
                        type: {
                            array: ['u8', 32],
                        },
                    },
                ],
            },
        },
        {
            name: 'PackedCompressedAccountWithMerkleContext',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'compressedAccount',
                        type: {
                            defined: 'CompressedAccount',
                        },
                    },
                    {
                        name: 'merkleContext',
                        type: {
                            defined: 'PackedMerkleContext',
                        },
                    },
                    {
                        name: 'rootIndex',
                        docs: [
                            'Index of root used in inclusion validity proof.',
                        ],
                        type: 'u16',
                    },
                ],
            },
        },
        {
            name: 'PackedMerkleContext',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'merkleTreePubkeyIndex',
                        type: 'u8',
                    },
                    {
                        name: 'nullifierQueuePubkeyIndex',
                        type: 'u8',
                    },
                    {
                        name: 'leafIndex',
                        type: 'u32',
                    },
                    {
                        name: 'queueIndex',
                        docs: [
                            'Index of leaf in queue. Placeholder of batched Merkle tree updates',
                            'currently unimplemented.',
                        ],
                        type: {
                            option: {
                                defined: 'QueueIndex',
                            },
                        },
                    },
                ],
            },
        },
        {
            name: 'QueueIndex',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'queueId',
                        docs: ['Id of queue in queue account.'],
                        type: 'u8',
                    },
                    {
                        name: 'index',
                        docs: ['Index of compressed account hash in queue.'],
                        type: 'u16',
                    },
                ],
            },
        },
        {
            name: 'MerkleTreeSequenceNumber',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'pubkey',
                        type: 'publicKey',
                    },
                    {
                        name: 'seq',
                        type: 'u64',
                    },
                ],
            },
        },
        {
            name: 'PublicTransactionEvent',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'inputCompressedAccountHashes',
                        type: {
                            vec: {
                                array: ['u8', 32],
                            },
                        },
                    },
                    {
                        name: 'outputCompressedAccountHashes',
                        type: {
                            vec: {
                                array: ['u8', 32],
                            },
                        },
                    },
                    {
                        name: 'outputCompressedAccounts',
                        type: {
                            vec: {
                                defined:
                                    'OutputCompressedAccountWithPackedContext',
                            },
                        },
                    },
                    {
                        name: 'outputLeafIndices',
                        type: {
                            vec: 'u32',
                        },
                    },
                    {
                        name: 'sequenceNumbers',
                        type: {
                            vec: {
                                defined: 'MerkleTreeSequenceNumber',
                            },
                        },
                    },
                    {
                        name: 'relayFee',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'isCompress',
                        type: 'bool',
                    },
                    {
                        name: 'compressOrDecompressLamports',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'pubkeyArray',
                        type: {
                            vec: 'publicKey',
                        },
                    },
                    {
                        name: 'message',
                        type: {
                            option: 'bytes',
                        },
                    },
                ],
            },
        },
    ],
    errors: [
        {
            code: 6000,
            name: 'SumCheckFailed',
            msg: 'Sum check failed',
        },
        {
            code: 6001,
            name: 'SignerCheckFailed',
            msg: 'Signer check failed',
        },
        {
            code: 6002,
            name: 'CpiSignerCheckFailed',
            msg: 'Cpi signer check failed',
        },
        {
            code: 6003,
            name: 'ComputeInputSumFailed',
            msg: 'Computing input sum failed.',
        },
        {
            code: 6004,
            name: 'ComputeOutputSumFailed',
            msg: 'Computing output sum failed.',
        },
        {
            code: 6005,
            name: 'ComputeRpcSumFailed',
            msg: 'Computing rpc sum failed.',
        },
        {
            code: 6006,
            name: 'InvalidNoopPubkey',
            msg: 'InvalidNoopPubkey',
        },
        {
            code: 6007,
            name: 'InvalidAddress',
            msg: 'InvalidAddress',
        },
        {
            code: 6008,
            name: 'DeriveAddressError',
            msg: 'DeriveAddressError',
        },
        {
            code: 6009,
            name: 'CompressedSolPdaUndefinedForCompressSol',
            msg: 'CompressedSolPdaUndefinedForCompressSol',
        },
        {
            code: 6010,
            name: 'DeCompressLamportsUndefinedForCompressSol',
            msg: 'DeCompressLamportsUndefinedForCompressSol',
        },
        {
            code: 6011,
            name: 'CompressedSolPdaUndefinedForDecompressSol',
            msg: 'CompressedSolPdaUndefinedForDecompressSol',
        },
        {
            code: 6012,
            name: 'DeCompressLamportsUndefinedForDecompressSol',
            msg: 'DeCompressLamportsUndefinedForDecompressSol',
        },
        {
            code: 6013,
            name: 'DecompressRecipientUndefinedForDecompressSol',
            msg: 'DecompressRecipientUndefinedForDecompressSol',
        },
        {
            code: 6014,
            name: 'WriteAccessCheckFailed',
            msg: 'WriteAccessCheckFailed',
        },
        {
            code: 6015,
            name: 'InvokingProgramNotProvided',
            msg: 'InvokingProgramNotProvided',
        },
        {
            code: 6016,
            name: 'InvalidCapacity',
            msg: 'InvalidCapacity',
        },
        {
            code: 6017,
            name: 'InvalidMerkleTreeOwner',
            msg: 'InvalidMerkleTreeOwner',
        },
        {
            code: 6018,
            name: 'ProofIsNone',
            msg: 'ProofIsNone',
        },
        {
            code: 6019,
            name: 'ProofIsSome',
            msg: 'Proof is some but no input compressed accounts or new addresses provided.',
        },
        {
            code: 6020,
            name: 'EmptyInputs',
            msg: 'EmptyInputs',
        },
        {
            code: 6021,
            name: 'CpiContextAccountUndefined',
            msg: 'CpiContextAccountUndefined',
        },
        {
            code: 6022,
            name: 'CpiContextEmpty',
            msg: 'CpiContextEmpty',
        },
        {
            code: 6023,
            name: 'CpiContextMissing',
            msg: 'CpiContextMissing',
        },
        {
            code: 6024,
            name: 'DecompressionRecipientDefined',
            msg: 'DecompressionRecipientDefined',
        },
        {
            code: 6025,
            name: 'SolPoolPdaDefined',
            msg: 'SolPoolPdaDefined',
        },
        {
            code: 6026,
            name: 'AppendStateFailed',
            msg: 'AppendStateFailed',
        },
        {
            code: 6027,
            name: 'InstructionNotCallable',
            msg: 'The instruction is not callable',
        },
        {
            code: 6028,
            name: 'CpiContextFeePayerMismatch',
            msg: 'CpiContextFeePayerMismatch',
        },
        {
            code: 6029,
            name: 'CpiContextAssociatedMerkleTreeMismatch',
            msg: 'CpiContextAssociatedMerkleTreeMismatch',
        },
        {
            code: 6030,
            name: 'NoInputs',
            msg: 'NoInputs',
        },
        {
            code: 6031,
            name: 'InputMerkleTreeIndicesNotInOrder',
            msg: 'Input merkle tree indices are not in ascending order.',
        },
        {
            code: 6032,
            name: 'OutputMerkleTreeIndicesNotInOrder',
            msg: 'Output merkle tree indices are not in ascending order.',
        },
    ],
};
