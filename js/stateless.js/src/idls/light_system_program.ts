/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/light_system_program.json`.
 */
export type LightSystemProgram = {
    address: '9yrxXVGVs7bbnvPXta77RdLSubNzft49vQ7vpyyh1w8N';
    metadata: {
        name: 'lightSystemProgram';
        version: '0.7.0';
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
                    docs: [
                        'Fee payer needs to be mutable to pay rollover and protocol fees.',
                    ];
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
                                    133,
                                    110,
                                    129,
                                    246,
                                    34,
                                    115,
                                    134,
                                    36,
                                    116,
                                    170,
                                    124,
                                    71,
                                    118,
                                    109,
                                    243,
                                    40,
                                    228,
                                    60,
                                    79,
                                    79,
                                    177,
                                    82,
                                    14,
                                    121,
                                    210,
                                    121,
                                    182,
                                    180,
                                    6,
                                    232,
                                    141,
                                    67,
                                ];
                            },
                        ];
                        program: {
                            kind: 'const';
                            value: [
                                42,
                                198,
                                44,
                                250,
                                21,
                                12,
                                97,
                                213,
                                145,
                                98,
                                1,
                                147,
                                196,
                                121,
                                169,
                                33,
                                248,
                                42,
                                217,
                                95,
                                192,
                                109,
                                80,
                                10,
                                145,
                                159,
                                227,
                                79,
                                164,
                                34,
                                182,
                                86,
                            ];
                        };
                    };
                },
                {
                    name: 'noopProgram';
                },
                {
                    name: 'accountCompressionAuthority';
                    docs: [
                        'This pda is used to invoke the account compression program.',
                    ];
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
                    docs: ['Merkle trees.'];
                    address: '3syPfxf7UXUoHiC7H6W6jLVXAWhcWKGSxXMpNcUgTkS1';
                },
                {
                    name: 'solPoolPda';
                    docs: [
                        'Sol pool pda is used to store the native sol that has been compressed.',
                        "It's only required when compressing or decompressing sol.",
                    ];
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
                    docs: [
                        'Only needs to be provided for decompression as a recipient for the',
                        'decompressed sol.',
                        'Compressed sol originate from authority.',
                    ];
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
                    docs: [
                        'Fee payer needs to be mutable to pay rollover and protocol fees.',
                    ];
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
                                    133,
                                    110,
                                    129,
                                    246,
                                    34,
                                    115,
                                    134,
                                    36,
                                    116,
                                    170,
                                    124,
                                    71,
                                    118,
                                    109,
                                    243,
                                    40,
                                    228,
                                    60,
                                    79,
                                    79,
                                    177,
                                    82,
                                    14,
                                    121,
                                    210,
                                    121,
                                    182,
                                    180,
                                    6,
                                    232,
                                    141,
                                    67,
                                ];
                            },
                        ];
                        program: {
                            kind: 'const';
                            value: [
                                42,
                                198,
                                44,
                                250,
                                21,
                                12,
                                97,
                                213,
                                145,
                                98,
                                1,
                                147,
                                196,
                                121,
                                169,
                                33,
                                248,
                                42,
                                217,
                                95,
                                192,
                                109,
                                80,
                                10,
                                145,
                                159,
                                227,
                                79,
                                164,
                                34,
                                182,
                                86,
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
                    address: '3syPfxf7UXUoHiC7H6W6jLVXAWhcWKGSxXMpNcUgTkS1';
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
        {
            name: 'stubIdlBuild';
            docs: [
                'This function is a stub to allow Anchor to include the input types in',
                'the IDL. It should not be included in production builds nor be called in',
                'practice.',
            ];
            discriminator: [118, 99, 238, 243, 8, 167, 251, 168];
            accounts: [
                {
                    name: 'feePayer';
                    docs: [
                        'Fee payer needs to be mutable to pay rollover and protocol fees.',
                    ];
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
                                    133,
                                    110,
                                    129,
                                    246,
                                    34,
                                    115,
                                    134,
                                    36,
                                    116,
                                    170,
                                    124,
                                    71,
                                    118,
                                    109,
                                    243,
                                    40,
                                    228,
                                    60,
                                    79,
                                    79,
                                    177,
                                    82,
                                    14,
                                    121,
                                    210,
                                    121,
                                    182,
                                    180,
                                    6,
                                    232,
                                    141,
                                    67,
                                ];
                            },
                        ];
                        program: {
                            kind: 'const';
                            value: [
                                42,
                                198,
                                44,
                                250,
                                21,
                                12,
                                97,
                                213,
                                145,
                                98,
                                1,
                                147,
                                196,
                                121,
                                169,
                                33,
                                248,
                                42,
                                217,
                                95,
                                192,
                                109,
                                80,
                                10,
                                145,
                                159,
                                227,
                                79,
                                164,
                                34,
                                182,
                                86,
                            ];
                        };
                    };
                },
                {
                    name: 'noopProgram';
                },
                {
                    name: 'accountCompressionAuthority';
                    docs: [
                        'This pda is used to invoke the account compression program.',
                    ];
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
                    docs: ['Merkle trees.'];
                    address: '3syPfxf7UXUoHiC7H6W6jLVXAWhcWKGSxXMpNcUgTkS1';
                },
                {
                    name: 'solPoolPda';
                    docs: [
                        'Sol pool pda is used to store the native sol that has been compressed.',
                        "It's only required when compressing or decompressing sol.",
                    ];
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
                    docs: [
                        'Only needs to be provided for decompression as a recipient for the',
                        'decompressed sol.',
                        'Compressed sol originate from authority.',
                    ];
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
                    name: 'inputs1';
                    type: {
                        defined: {
                            name: 'instructionDataInvoke';
                        };
                    };
                },
                {
                    name: 'inputs2';
                    type: {
                        defined: {
                            name: 'instructionDataInvokeCpi';
                        };
                    };
                },
                {
                    name: 'inputs3';
                    type: {
                        defined: {
                            name: 'publicTransactionEvent';
                        };
                    };
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
            name: 'invalidAddress';
            msg: 'invalidAddress';
        },
        {
            code: 6007;
            name: 'deriveAddressError';
            msg: 'deriveAddressError';
        },
        {
            code: 6008;
            name: 'compressedSolPdaUndefinedForCompressSol';
            msg: 'compressedSolPdaUndefinedForCompressSol';
        },
        {
            code: 6009;
            name: 'deCompressLamportsUndefinedForCompressSol';
            msg: 'deCompressLamportsUndefinedForCompressSol';
        },
        {
            code: 6010;
            name: 'compressedSolPdaUndefinedForDecompressSol';
            msg: 'compressedSolPdaUndefinedForDecompressSol';
        },
        {
            code: 6011;
            name: 'deCompressLamportsUndefinedForDecompressSol';
            msg: 'deCompressLamportsUndefinedForDecompressSol';
        },
        {
            code: 6012;
            name: 'decompressRecipientUndefinedForDecompressSol';
            msg: 'decompressRecipientUndefinedForDecompressSol';
        },
        {
            code: 6013;
            name: 'writeAccessCheckFailed';
            msg: 'writeAccessCheckFailed';
        },
        {
            code: 6014;
            name: 'invokingProgramNotProvided';
            msg: 'invokingProgramNotProvided';
        },
        {
            code: 6015;
            name: 'invalidCapacity';
            msg: 'invalidCapacity';
        },
        {
            code: 6016;
            name: 'invalidMerkleTreeOwner';
            msg: 'invalidMerkleTreeOwner';
        },
        {
            code: 6017;
            name: 'proofIsNone';
            msg: 'proofIsNone';
        },
        {
            code: 6018;
            name: 'proofIsSome';
            msg: 'Proof is some but no input compressed accounts or new addresses provided.';
        },
        {
            code: 6019;
            name: 'emptyInputs';
            msg: 'emptyInputs';
        },
        {
            code: 6020;
            name: 'cpiContextAccountUndefined';
            msg: 'cpiContextAccountUndefined';
        },
        {
            code: 6021;
            name: 'cpiContextEmpty';
            msg: 'cpiContextEmpty';
        },
        {
            code: 6022;
            name: 'cpiContextMissing';
            msg: 'cpiContextMissing';
        },
        {
            code: 6023;
            name: 'decompressionRecipientDefined';
            msg: 'decompressionRecipientDefined';
        },
        {
            code: 6024;
            name: 'solPoolPdaDefined';
            msg: 'solPoolPdaDefined';
        },
        {
            code: 6025;
            name: 'appendStateFailed';
            msg: 'appendStateFailed';
        },
        {
            code: 6026;
            name: 'instructionNotCallable';
            msg: 'The instruction is not callable';
        },
        {
            code: 6027;
            name: 'cpiContextFeePayerMismatch';
            msg: 'cpiContextFeePayerMismatch';
        },
        {
            code: 6028;
            name: 'cpiContextAssociatedMerkleTreeMismatch';
            msg: 'cpiContextAssociatedMerkleTreeMismatch';
        },
        {
            code: 6029;
            name: 'noInputs';
            msg: 'noInputs';
        },
        {
            code: 6030;
            name: 'inputMerkleTreeIndicesNotInOrder';
            msg: 'Input merkle tree indices are not in ascending order.';
        },
        {
            code: 6031;
            name: 'outputMerkleTreeIndicesNotInOrder';
            msg: 'Output merkle tree indices are not in ascending order.';
        },
        {
            code: 6032;
            name: 'outputMerkleTreeNotUnique';
        },
        {
            code: 6033;
            name: 'dataFieldUndefined';
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
                            'Program owner of the Merkle tree. This will be used for program owned Merkle trees.',
                        ];
                        type: 'pubkey';
                    },
                    {
                        name: 'forester';
                        docs: [
                            'Optional privileged forester pubkey, can be set for custom Merkle trees',
                            'without a network fee. Merkle trees without network fees are not',
                            'forested by light foresters. The variable is not used in the account',
                            'compression program but the registry program. The registry program',
                            'implements access control to prevent contention during forester. The',
                            'forester pubkey specified in this struct can bypass contention checks.',
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
            name: 'instructionDataInvoke';
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
            name: 'merkleTreeSequenceNumber';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'pubkey';
                        type: 'pubkey';
                    },
                    {
                        name: 'seq';
                        type: 'u64';
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
                    {
                        name: 'readOnly';
                        docs: [
                            'Placeholder to mark accounts read-only unimplemented set to false.',
                        ];
                        type: 'bool';
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
                    {
                        name: 'queueIndex';
                        docs: [
                            'Index of leaf in queue. Placeholder of batched Merkle tree updates',
                            'currently unimplemented.',
                        ];
                        type: {
                            option: {
                                defined: {
                                    name: 'queueIndex';
                                };
                            };
                        };
                    },
                ];
            };
        },
        {
            name: 'publicTransactionEvent';
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
                                defined: {
                                    name: 'outputCompressedAccountWithPackedContext';
                                };
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
                                defined: {
                                    name: 'merkleTreeSequenceNumber';
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
                            vec: 'pubkey';
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
            name: 'queueIndex';
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
                    {
                        name: 'additionalBytes';
                        docs: [
                            'Placeholder for bytes of additional accounts which are tied to the',
                            'Merkle trees operation and need to be rolled over as well.',
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
