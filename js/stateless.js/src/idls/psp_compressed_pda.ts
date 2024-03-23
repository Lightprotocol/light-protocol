export type PspCompressedPda = {
    version: '0.3.0';
    name: 'psp_compressed_pda';
    instructions: [
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
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'noopProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'pspAccountCompressionAuthority';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'cpiSignatureAccount';
                    isMut: false;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'invokingProgram';
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
                        name: 'slot';
                        type: 'u64';
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
                        name: 'indexMtAccount';
                        type: 'u8';
                    },
                    {
                        name: 'indexNullifierArrayAccount';
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
                            option: 'publicKey';
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
                        name: 'outputAccountHashes';
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
                        name: 'deCompressAmount';
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
            docs: ['(swen): as type into IDL'];
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
    ];
};

export const IDL: PspCompressedPda = {
    version: '0.3.0',
    name: 'psp_compressed_pda',
    instructions: [
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
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'noopProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'pspAccountCompressionAuthority',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'cpiSignatureAccount',
                    isMut: false,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'invokingProgram',
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
                        name: 'slot',
                        type: 'u64',
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
                        name: 'indexMtAccount',
                        type: 'u8',
                    },
                    {
                        name: 'indexNullifierArrayAccount',
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
                            option: 'publicKey',
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
                        name: 'outputAccountHashes',
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
                        name: 'deCompressAmount',
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
            docs: ['(swen): as type into IDL'],
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
    ],
};
