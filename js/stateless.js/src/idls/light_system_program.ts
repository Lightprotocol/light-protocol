/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/light_system_program.json`.
 */
export type LightSystemProgram = {
    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL';
    metadata: {
        name: 'lightSystemProgram';
        version: '0.4.1';
        spec: '0.1.0';
        description: 'ZK Compression on Solana';
        repository: 'https://github.com/Lightprotocol/light-protocol';
    };
    instructions: [
        {
            name: 'initCpiContextAccount';
            discriminator: [233, 112, 71, 66, 121, 33, 178, 188];
            accounts: [
                {
                    name: 'feePayer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'cpiContextAccount';
                    writable: true;
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
                {
                    name: 'associatedMerkleTree';
                },
            ];
            args: [];
        },
        {
            name: 'invoke';
            discriminator: [26, 16, 169, 7, 21, 202, 242, 25];
            accounts: [
                {
                    name: 'feePayer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'registeredProgramPda';
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    122,
                                    247,
                                    228,
                                    253,
                                    125,
                                    235,
                                    168,
                                    178,
                                    118,
                                    247,
                                    148,
                                    132,
                                    87,
                                    69,
                                    138,
                                    249,
                                    48,
                                    38,
                                    162,
                                    157,
                                    101,
                                    94,
                                    204,
                                    195,
                                    59,
                                    23,
                                    183,
                                    138,
                                    180,
                                    125,
                                    89,
                                    223,
                                ];
                            },
                        ];
                        program: {
                            kind: 'const';
                            value: [
                                5,
                                41,
                                183,
                                51,
                                223,
                                226,
                                6,
                                143,
                                39,
                                89,
                                245,
                                240,
                                191,
                                162,
                                53,
                                191,
                                118,
                                47,
                                134,
                                180,
                                213,
                                25,
                                153,
                                162,
                                127,
                                46,
                                146,
                                16,
                                126,
                                15,
                                255,
                                67,
                            ];
                        };
                    };
                },
                {
                    name: 'noopProgram';
                },
                {
                    name: 'accountCompressionAuthority';
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    99,
                                    112,
                                    105,
                                    95,
                                    97,
                                    117,
                                    116,
                                    104,
                                    111,
                                    114,
                                    105,
                                    116,
                                    121,
                                ];
                            },
                        ];
                    };
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'solPoolPda';
                    writable: true;
                    optional: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    115,
                                    111,
                                    108,
                                    95,
                                    112,
                                    111,
                                    111,
                                    108,
                                    95,
                                    112,
                                    100,
                                    97,
                                ];
                            },
                        ];
                    };
                },
                {
                    name: 'decompressionRecipient';
                    writable: true;
                    optional: true;
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
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
            discriminator: [49, 212, 191, 129, 39, 194, 43, 196];
            accounts: [
                {
                    name: 'feePayer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'registeredProgramPda';
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    122,
                                    247,
                                    228,
                                    253,
                                    125,
                                    235,
                                    168,
                                    178,
                                    118,
                                    247,
                                    148,
                                    132,
                                    87,
                                    69,
                                    138,
                                    249,
                                    48,
                                    38,
                                    162,
                                    157,
                                    101,
                                    94,
                                    204,
                                    195,
                                    59,
                                    23,
                                    183,
                                    138,
                                    180,
                                    125,
                                    89,
                                    223,
                                ];
                            },
                        ];
                        program: {
                            kind: 'const';
                            value: [
                                5,
                                41,
                                183,
                                51,
                                223,
                                226,
                                6,
                                143,
                                39,
                                89,
                                245,
                                240,
                                191,
                                162,
                                53,
                                191,
                                118,
                                47,
                                134,
                                180,
                                213,
                                25,
                                153,
                                162,
                                127,
                                46,
                                146,
                                16,
                                126,
                                15,
                                255,
                                67,
                            ];
                        };
                    };
                },
                {
                    name: 'noopProgram';
                },
                {
                    name: 'accountCompressionAuthority';
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    99,
                                    112,
                                    105,
                                    95,
                                    97,
                                    117,
                                    116,
                                    104,
                                    111,
                                    114,
                                    105,
                                    116,
                                    121,
                                ];
                            },
                        ];
                    };
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'invokingProgram';
                },
                {
                    name: 'solPoolPda';
                    writable: true;
                    optional: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    115,
                                    111,
                                    108,
                                    95,
                                    112,
                                    111,
                                    111,
                                    108,
                                    95,
                                    112,
                                    100,
                                    97,
                                ];
                            },
                        ];
                    };
                },
                {
                    name: 'decompressionRecipient';
                    writable: true;
                    optional: true;
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
                {
                    name: 'cpiContextAccount';
                    writable: true;
                    optional: true;
                },
            ];
            args: [
                {
                    name: 'inputs';
                    type: 'bytes';
                },
            ];
        },
    ];
    accounts: [
        {
            name: 'cpiContextAccount';
            discriminator: [22, 20, 149, 218, 74, 204, 128, 166];
        },
        {
            name: 'registeredProgram';
            discriminator: [31, 251, 180, 235, 3, 116, 50, 4];
        },
        {
            name: 'stateMerkleTreeAccount';
            discriminator: [172, 43, 172, 186, 29, 73, 219, 84];
        },
    ];
    errors: [
        {
            code: 6000;
            name: 'sumCheckFailed';
            msg: 'Sum check failed';
        },
        {
            code: 6001;
            name: 'signerCheckFailed';
            msg: 'Signer check failed';
        },
        {
            code: 6002;
            name: 'cpiSignerCheckFailed';
            msg: 'Cpi signer check failed';
        },
        {
            code: 6003;
            name: 'computeInputSumFailed';
            msg: 'Computing input sum failed.';
        },
        {
            code: 6004;
            name: 'computeOutputSumFailed';
            msg: 'Computing output sum failed.';
        },
        {
            code: 6005;
            name: 'computeRpcSumFailed';
            msg: 'Computing rpc sum failed.';
        },
        {
            code: 6006;
            name: 'invalidNoopPubkey';
            msg: 'invalidNoopPubkey';
        },
        {
            code: 6007;
            name: 'invalidAddress';
            msg: 'invalidAddress';
        },
        {
            code: 6008;
            name: 'deriveAddressError';
            msg: 'deriveAddressError';
        },
        {
            code: 6009;
            name: 'compressedSolPdaUndefinedForCompressSol';
            msg: 'compressedSolPdaUndefinedForCompressSol';
        },
        {
            code: 6010;
            name: 'deCompressLamportsUndefinedForCompressSol';
            msg: 'deCompressLamportsUndefinedForCompressSol';
        },
        {
            code: 6011;
            name: 'compressedSolPdaUndefinedForDecompressSol';
            msg: 'compressedSolPdaUndefinedForDecompressSol';
        },
        {
            code: 6012;
            name: 'deCompressLamportsUndefinedForDecompressSol';
            msg: 'deCompressLamportsUndefinedForDecompressSol';
        },
        {
            code: 6013;
            name: 'decompressRecipientUndefinedForDecompressSol';
            msg: 'decompressRecipientUndefinedForDecompressSol';
        },
        {
            code: 6014;
            name: 'writeAccessCheckFailed';
            msg: 'writeAccessCheckFailed';
        },
        {
            code: 6015;
            name: 'invokingProgramNotProvided';
            msg: 'invokingProgramNotProvided';
        },
        {
            code: 6016;
            name: 'invalidCapacity';
            msg: 'invalidCapacity';
        },
        {
            code: 6017;
            name: 'invalidMerkleTreeOwner';
            msg: 'invalidMerkleTreeOwner';
        },
        {
            code: 6018;
            name: 'proofIsNone';
            msg: 'proofIsNone';
        },
        {
            code: 6019;
            name: 'proofIsSome';
            msg: 'proofIsSome';
        },
        {
            code: 6020;
            name: 'emptyInputs';
            msg: 'emptyInputs';
        },
        {
            code: 6021;
            name: 'cpiContextAccountUndefined';
            msg: 'cpiContextAccountUndefined';
        },
        {
            code: 6022;
            name: 'cpiContextProofMismatch';
            msg: 'cpiContextMismatch';
        },
        {
            code: 6023;
            name: 'cpiContextEmpty';
            msg: 'cpiContextEmpty';
        },
        {
            code: 6024;
            name: 'cpiContextMissing';
            msg: 'cpiContextMissing';
        },
        {
            code: 6025;
            name: 'decompressionRecipienDefined';
            msg: 'decompressionRecipienDefined';
        },
        {
            code: 6026;
            name: 'solPoolPdaDefined';
            msg: 'solPoolPdaDefined';
        },
        {
            code: 6027;
            name: 'appendStateFailed';
            msg: 'appendStateFailed';
        },
        {
            code: 6028;
            name: 'instructionNotCallable';
            msg: 'The instruction is not callable';
        },
        {
            code: 6029;
            name: 'cpiContextFeePayerMismatch';
            msg: 'cpiContextFeePayerMismatch';
        },
    ];
    types: [
        {
            name: 'accessMetadata';
            serialization: 'bytemuck';
            repr: {
                kind: 'c';
            };
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'owner';
                        docs: ['Owner of the Merkle tree.'];
                        type: 'pubkey';
                    },
                    {
                        name: 'programOwner';
                        docs: [
                            'Delegate of the Merkle tree. This will be used for program owned Merkle trees.',
                        ];
                        type: 'pubkey';
                    },
                ];
            };
        },
        {
            name: 'compressedAccount';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'owner';
                        type: 'pubkey';
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
                                defined: {
                                    name: 'compressedAccountData';
                                };
                            };
                        };
                    },
                ];
            };
        },
        {
            name: 'compressedAccountData';
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
            name: 'compressedCpiContext';
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
            name: 'compressedProof';
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
                        type: 'pubkey';
                    },
                    {
                        name: 'associatedMerkleTree';
                        type: 'pubkey';
                    },
                    {
                        name: 'context';
                        type: {
                            vec: {
                                defined: {
                                    name: 'instructionDataInvokeCpi';
                                };
                            };
                        };
                    },
                ];
            };
        },
        {
            name: 'instructionDataInvokeCpi';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'proof';
                        type: {
                            option: {
                                defined: {
                                    name: 'compressedProof';
                                };
                            };
                        };
                    },
                    {
                        name: 'newAddressParams';
                        type: {
                            vec: {
                                defined: {
                                    name: 'newAddressParamsPacked';
                                };
                            };
                        };
                    },
                    {
                        name: 'inputCompressedAccountsWithMerkleContext';
                        type: {
                            vec: {
                                defined: {
                                    name: 'packedCompressedAccountWithMerkleContext';
                                };
                            };
                        };
                    },
                    {
                        name: 'outputCompressedAccounts';
                        type: {
                            vec: {
                                defined: {
                                    name: 'outputCompressedAccountWithPackedContext';
                                };
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
                                defined: {
                                    name: 'compressedCpiContext';
                                };
                            };
                        };
                    },
                ];
            };
        },
        {
            name: 'merkleTreeMetadata';
            serialization: 'bytemuck';
            repr: {
                kind: 'c';
            };
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'accessMetadata';
                        type: {
                            defined: {
                                name: 'accessMetadata';
                            };
                        };
                    },
                    {
                        name: 'rolloverMetadata';
                        type: {
                            defined: {
                                name: 'rolloverMetadata';
                            };
                        };
                    },
                    {
                        name: 'associatedQueue';
                        type: 'pubkey';
                    },
                    {
                        name: 'nextMerkleTree';
                        type: 'pubkey';
                    },
                ];
            };
        },
        {
            name: 'newAddressParamsPacked';
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
            name: 'outputCompressedAccountWithPackedContext';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'compressedAccount';
                        type: {
                            defined: {
                                name: 'compressedAccount';
                            };
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
            name: 'packedCompressedAccountWithMerkleContext';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'compressedAccount';
                        type: {
                            defined: {
                                name: 'compressedAccount';
                            };
                        };
                    },
                    {
                        name: 'merkleContext';
                        type: {
                            defined: {
                                name: 'packedMerkleContext';
                            };
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
            name: 'packedMerkleContext';
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
                ];
            };
        },
        {
            name: 'registeredProgram';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'registeredProgramId';
                        type: 'pubkey';
                    },
                    {
                        name: 'groupAuthorityPda';
                        type: 'pubkey';
                    },
                ];
            };
        },
        {
            name: 'rolloverMetadata';
            serialization: 'bytemuck';
            repr: {
                kind: 'c';
            };
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
            name: 'stateMerkleTreeAccount';
            docs: [
                'Concurrent state Merkle tree used for public compressed transactions.',
            ];
            serialization: 'bytemuck';
            repr: {
                kind: 'c';
            };
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'metadata';
                        type: {
                            defined: {
                                name: 'merkleTreeMetadata';
                            };
                        };
                    },
                ];
            };
        },
    ];
    constants: [
        {
            name: 'solPoolPdaSeed';
            type: 'bytes';
            value: '[115, 111, 108, 95, 112, 111, 111, 108, 95, 112, 100, 97]';
        },
    ];
};
