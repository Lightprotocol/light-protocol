export type PspCompressedPda = {
    version: '0.3.0';
    name: 'psp_compressed_pda';
    constants: [
        {
            name: 'COMPRESSED_SOL_PDA_SEED';
            type: 'bytes';
            value: '[99, 111, 109, 112, 114, 101, 115, 115, 101, 100, 95, 115, 111, 108, 95, 112, 100, 97]';
        },
    ];
    instructions: [
        {
            name: 'initCompressSolPda';
            docs: [
                'Initializes the compressed sol pda.',
                'This pda is used to store compressed sol for the protocol.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'compressedSolPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [];
        },
        {
            name: 'initCpiSignatureAccount';
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'cpiSignatureAccount';
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
            name: 'executeCompressedTransaction';
            docs: [
                'This function can be used to transfer sol and execute any other compressed transaction.',
                'Instruction data is not optimized for space.',
                'This method can be called by cpi so that instruction data can be compressed with a custom algorithm.',
            ];
            accounts: [
                {
                    name: 'signer';
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
                    name: 'pspAccountCompressionAuthority';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'cpiSignatureAccount';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'invokingProgram';
                    isMut: false;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'compressedSolPda';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'compressionRecipient';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                    isOptional: true;
                },
            ];
            args: [
                {
                    name: 'inputs';
                    type: 'bytes';
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
            returns: {
                defined: 'crate::event::PublicTransactionEvent';
            };
        },
    ];
    accounts: [
        {
            name: 'cpiSignatureAccount';
            docs: [
                'collects invocations without proofs',
                'invocations are collected and processed when an invocation with a proof is received',
            ];
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'associatedMerkleTree';
                        type: 'publicKey';
                    },
                    {
                        name: 'execute';
                        type: 'bool';
                    },
                    {
                        name: 'signatures';
                        type: {
                            vec: {
                                defined: 'InstructionDataTransfer';
                            };
                        };
                    },
                ];
            };
        },
        {
            name: 'compressedSolPda';
            type: {
                kind: 'struct';
                fields: [];
            };
        },
    ];
    types: [
        {
            name: 'CompressedAccountWithMerkleContext';
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
            name: 'CompressedCpiContext';
            docs: ['To spend multiple compressed'];
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'cpiSignatureAccountIndex';
                        docs: [
                            'index of the output state Merkle tree that will be used to store cpi signatures',
                            'The transaction will fail if this index is not consistent in your transaction.',
                        ];
                        type: 'u8';
                    },
                    {
                        name: 'execute';
                        docs: [
                            'The final cpi of your program needs to set execute to true.',
                            'Execute compressed transaction will verify the proof and execute the transaction if this is true.',
                            'If this is false the transaction will be stored in the cpi signature account.',
                        ];
                        type: 'bool';
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
                        name: 'inputCompressedAccounts';
                        type: {
                            vec: {
                                defined: 'CompressedAccountWithMerkleContext';
                            };
                        };
                    },
                    {
                        name: 'outputCompressedAccounts';
                        type: {
                            vec: {
                                defined: 'CompressedAccount';
                            };
                        };
                    },
                    {
                        name: 'outputStateMerkleTreeAccountIndices';
                        type: 'bytes';
                    },
                    {
                        name: 'outputLeafIndices';
                        type: {
                            vec: 'u32';
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
                        name: 'compressionLamports';
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
        {
            name: 'InstructionDataTransfer';
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
                        name: 'inputRootIndices';
                        type: {
                            vec: 'u16';
                        };
                    },
                    {
                        name: 'inputCompressedAccountsWithMerkleContext';
                        type: {
                            vec: {
                                defined: 'CompressedAccountWithMerkleContext';
                            };
                        };
                    },
                    {
                        name: 'outputCompressedAccounts';
                        type: {
                            vec: {
                                defined: 'CompressedAccount';
                            };
                        };
                    },
                    {
                        name: 'outputStateMerkleTreeAccountIndices';
                        docs: [
                            'The indices of the accounts in the output state merkle tree.',
                        ];
                        type: 'bytes';
                    },
                    {
                        name: 'relayFee';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'compressionLamports';
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
                            option: {
                                vec: 'bytes';
                            };
                        };
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
            name: 'NewAddressParams';
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
                        name: 'addressQueuePubkey';
                        type: 'publicKey';
                    },
                    {
                        name: 'addressMerkleTreePubkey';
                        type: 'publicKey';
                    },
                    {
                        name: 'addressMerkleTreeRootIndex';
                        type: 'u16';
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
            name: 'InUtxosAlreadyAdded';
            msg: 'InUtxosAlreadyAdded';
        },
        {
            code: 6007;
            name: 'NumberOfLeavesMissmatch';
            msg: 'NumberOfLeavesMissmatch';
        },
        {
            code: 6008;
            name: 'MerkleTreePubkeysMissmatch';
            msg: 'MerkleTreePubkeysMissmatch';
        },
        {
            code: 6009;
            name: 'NullifierArrayPubkeysMissmatch';
            msg: 'NullifierArrayPubkeysMissmatch';
        },
        {
            code: 6010;
            name: 'InvalidNoopPubkey';
            msg: 'InvalidNoopPubkey';
        },
        {
            code: 6011;
            name: 'InvalidPublicInputsLength';
            msg: 'InvalidPublicInputsLength';
        },
        {
            code: 6012;
            name: 'DecompressG1Failed';
            msg: 'Decompress G1 Failed';
        },
        {
            code: 6013;
            name: 'DecompressG2Failed';
            msg: 'Decompress G2 Failed';
        },
        {
            code: 6014;
            name: 'CreateGroth16VerifierFailed';
            msg: 'CreateGroth16VerifierFailed';
        },
        {
            code: 6015;
            name: 'ProofVerificationFailed';
            msg: 'ProofVerificationFailed';
        },
        {
            code: 6016;
            name: 'PublicInputsTryIntoFailed';
            msg: 'PublicInputsTryIntoFailed';
        },
        {
            code: 6017;
            name: 'CompressedAccountHashError';
            msg: 'CompressedAccountHashError';
        },
        {
            code: 6018;
            name: 'InvalidAddress';
            msg: 'InvalidAddress';
        },
        {
            code: 6019;
            name: 'InvalidAddressQueue';
            msg: 'InvalidAddressQueue';
        },
        {
            code: 6020;
            name: 'InvalidNullifierQueue';
            msg: 'InvalidNullifierQueue';
        },
        {
            code: 6021;
            name: 'DeriveAddressError';
            msg: 'DeriveAddressError';
        },
        {
            code: 6022;
            name: 'CompressSolTransferFailed';
            msg: 'CompressSolTransferFailed';
        },
        {
            code: 6023;
            name: 'CompressedSolPdaUndefinedForCompressSol';
            msg: 'CompressedSolPdaUndefinedForCompressSol';
        },
        {
            code: 6024;
            name: 'DeCompressLamportsUndefinedForCompressSol';
            msg: 'DeCompressLamportsUndefinedForCompressSol';
        },
        {
            code: 6025;
            name: 'CompressedSolPdaUndefinedForDecompressSol';
            msg: 'CompressedSolPdaUndefinedForDecompressSol';
        },
        {
            code: 6026;
            name: 'DeCompressLamportsUndefinedForDecompressSol';
            msg: 'DeCompressLamportsUndefinedForDecompressSol';
        },
        {
            code: 6027;
            name: 'DecompressRecipientUndefinedForDecompressSol';
            msg: 'DecompressRecipientUndefinedForDecompressSol';
        },
        {
            code: 6028;
            name: 'LengthMismatch';
            msg: 'LengthMismatch';
        },
        {
            code: 6029;
            name: 'DelegateUndefined';
            msg: 'DelegateUndefined while delegated amount is defined';
        },
        {
            code: 6030;
            name: 'CpiSignatureAccountUndefined';
            msg: 'CpiSignatureAccountUndefined';
        },
        {
            code: 6031;
            name: 'WriteAccessCheckFailed';
            msg: 'WriteAccessCheckFailed';
        },
    ];
};

export const IDL: PspCompressedPda = {
    version: '0.3.0',
    name: 'psp_compressed_pda',
    constants: [
        {
            name: 'COMPRESSED_SOL_PDA_SEED',
            type: 'bytes',
            value: '[99, 111, 109, 112, 114, 101, 115, 115, 101, 100, 95, 115, 111, 108, 95, 112, 100, 97]',
        },
    ],
    instructions: [
        {
            name: 'initCompressSolPda',
            docs: [
                'Initializes the compressed sol pda.',
                'This pda is used to store compressed sol for the protocol.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'compressedSolPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [],
        },
        {
            name: 'initCpiSignatureAccount',
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'cpiSignatureAccount',
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
            name: 'executeCompressedTransaction',
            docs: [
                'This function can be used to transfer sol and execute any other compressed transaction.',
                'Instruction data is not optimized for space.',
                'This method can be called by cpi so that instruction data can be compressed with a custom algorithm.',
            ],
            accounts: [
                {
                    name: 'signer',
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
                    name: 'pspAccountCompressionAuthority',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'cpiSignatureAccount',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'invokingProgram',
                    isMut: false,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'compressedSolPda',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'compressionRecipient',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                    isOptional: true,
                },
            ],
            args: [
                {
                    name: 'inputs',
                    type: 'bytes',
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
            returns: {
                defined: 'crate::event::PublicTransactionEvent',
            },
        },
    ],
    accounts: [
        {
            name: 'cpiSignatureAccount',
            docs: [
                'collects invocations without proofs',
                'invocations are collected and processed when an invocation with a proof is received',
            ],
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'associatedMerkleTree',
                        type: 'publicKey',
                    },
                    {
                        name: 'execute',
                        type: 'bool',
                    },
                    {
                        name: 'signatures',
                        type: {
                            vec: {
                                defined: 'InstructionDataTransfer',
                            },
                        },
                    },
                ],
            },
        },
        {
            name: 'compressedSolPda',
            type: {
                kind: 'struct',
                fields: [],
            },
        },
    ],
    types: [
        {
            name: 'CompressedAccountWithMerkleContext',
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
            name: 'CompressedCpiContext',
            docs: ['To spend multiple compressed'],
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'cpiSignatureAccountIndex',
                        docs: [
                            'index of the output state Merkle tree that will be used to store cpi signatures',
                            'The transaction will fail if this index is not consistent in your transaction.',
                        ],
                        type: 'u8',
                    },
                    {
                        name: 'execute',
                        docs: [
                            'The final cpi of your program needs to set execute to true.',
                            'Execute compressed transaction will verify the proof and execute the transaction if this is true.',
                            'If this is false the transaction will be stored in the cpi signature account.',
                        ],
                        type: 'bool',
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
                        name: 'inputCompressedAccounts',
                        type: {
                            vec: {
                                defined: 'CompressedAccountWithMerkleContext',
                            },
                        },
                    },
                    {
                        name: 'outputCompressedAccounts',
                        type: {
                            vec: {
                                defined: 'CompressedAccount',
                            },
                        },
                    },
                    {
                        name: 'outputStateMerkleTreeAccountIndices',
                        type: 'bytes',
                    },
                    {
                        name: 'outputLeafIndices',
                        type: {
                            vec: 'u32',
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
                        name: 'compressionLamports',
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
        {
            name: 'InstructionDataTransfer',
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
                        name: 'inputRootIndices',
                        type: {
                            vec: 'u16',
                        },
                    },
                    {
                        name: 'inputCompressedAccountsWithMerkleContext',
                        type: {
                            vec: {
                                defined: 'CompressedAccountWithMerkleContext',
                            },
                        },
                    },
                    {
                        name: 'outputCompressedAccounts',
                        type: {
                            vec: {
                                defined: 'CompressedAccount',
                            },
                        },
                    },
                    {
                        name: 'outputStateMerkleTreeAccountIndices',
                        docs: [
                            'The indices of the accounts in the output state merkle tree.',
                        ],
                        type: 'bytes',
                    },
                    {
                        name: 'relayFee',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'compressionLamports',
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
                            option: {
                                vec: 'bytes',
                            },
                        },
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
            name: 'NewAddressParams',
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
                        name: 'addressQueuePubkey',
                        type: 'publicKey',
                    },
                    {
                        name: 'addressMerkleTreePubkey',
                        type: 'publicKey',
                    },
                    {
                        name: 'addressMerkleTreeRootIndex',
                        type: 'u16',
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
            name: 'InUtxosAlreadyAdded',
            msg: 'InUtxosAlreadyAdded',
        },
        {
            code: 6007,
            name: 'NumberOfLeavesMissmatch',
            msg: 'NumberOfLeavesMissmatch',
        },
        {
            code: 6008,
            name: 'MerkleTreePubkeysMissmatch',
            msg: 'MerkleTreePubkeysMissmatch',
        },
        {
            code: 6009,
            name: 'NullifierArrayPubkeysMissmatch',
            msg: 'NullifierArrayPubkeysMissmatch',
        },
        {
            code: 6010,
            name: 'InvalidNoopPubkey',
            msg: 'InvalidNoopPubkey',
        },
        {
            code: 6011,
            name: 'InvalidPublicInputsLength',
            msg: 'InvalidPublicInputsLength',
        },
        {
            code: 6012,
            name: 'DecompressG1Failed',
            msg: 'Decompress G1 Failed',
        },
        {
            code: 6013,
            name: 'DecompressG2Failed',
            msg: 'Decompress G2 Failed',
        },
        {
            code: 6014,
            name: 'CreateGroth16VerifierFailed',
            msg: 'CreateGroth16VerifierFailed',
        },
        {
            code: 6015,
            name: 'ProofVerificationFailed',
            msg: 'ProofVerificationFailed',
        },
        {
            code: 6016,
            name: 'PublicInputsTryIntoFailed',
            msg: 'PublicInputsTryIntoFailed',
        },
        {
            code: 6017,
            name: 'CompressedAccountHashError',
            msg: 'CompressedAccountHashError',
        },
        {
            code: 6018,
            name: 'InvalidAddress',
            msg: 'InvalidAddress',
        },
        {
            code: 6019,
            name: 'InvalidAddressQueue',
            msg: 'InvalidAddressQueue',
        },
        {
            code: 6020,
            name: 'InvalidNullifierQueue',
            msg: 'InvalidNullifierQueue',
        },
        {
            code: 6021,
            name: 'DeriveAddressError',
            msg: 'DeriveAddressError',
        },
        {
            code: 6022,
            name: 'CompressSolTransferFailed',
            msg: 'CompressSolTransferFailed',
        },
        {
            code: 6023,
            name: 'CompressedSolPdaUndefinedForCompressSol',
            msg: 'CompressedSolPdaUndefinedForCompressSol',
        },
        {
            code: 6024,
            name: 'DeCompressLamportsUndefinedForCompressSol',
            msg: 'DeCompressLamportsUndefinedForCompressSol',
        },
        {
            code: 6025,
            name: 'CompressedSolPdaUndefinedForDecompressSol',
            msg: 'CompressedSolPdaUndefinedForDecompressSol',
        },
        {
            code: 6026,
            name: 'DeCompressLamportsUndefinedForDecompressSol',
            msg: 'DeCompressLamportsUndefinedForDecompressSol',
        },
        {
            code: 6027,
            name: 'DecompressRecipientUndefinedForDecompressSol',
            msg: 'DecompressRecipientUndefinedForDecompressSol',
        },
        {
            code: 6028,
            name: 'LengthMismatch',
            msg: 'LengthMismatch',
        },
        {
            code: 6029,
            name: 'DelegateUndefined',
            msg: 'DelegateUndefined while delegated amount is defined',
        },
        {
            code: 6030,
            name: 'CpiSignatureAccountUndefined',
            msg: 'CpiSignatureAccountUndefined',
        },
        {
            code: 6031,
            name: 'WriteAccessCheckFailed',
            msg: 'WriteAccessCheckFailed',
        },
    ],
};
