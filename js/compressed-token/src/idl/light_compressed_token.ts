export type LightCompressedToken = {
    version: '0.4.1';
    name: 'light_compressed_token';
    instructions: [
        {
            name: 'createTokenPool';
            docs: [
                'This instruction expects a mint account to be created in a separate',
                'token program instruction with token authority as mint authority. This',
                'instruction creates a token pool account for that mint owned by token',
                'authority.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'tokenPoolPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'mint';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'tokenProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'cpiAuthorityPda';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [];
        },
        {
            name: 'mintTo';
            docs: [
                'Mints tokens from an spl token mint to a list of compressed accounts.',
                'Minted tokens are transferred to a pool account owned by the compressed',
                'token program. The instruction creates one compressed output account for',
                'every amount and pubkey input pair one output compressed account.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthorityPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'mint';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'tokenPoolPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'tokenProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'lightSystemProgram';
                    isMut: false;
                    isSigner: false;
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
                    name: 'merkleTree';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'selfProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'publicKeys';
                    type: {
                        vec: 'publicKey';
                    };
                },
                {
                    name: 'amounts';
                    type: {
                        vec: 'u64';
                    };
                },
            ];
        },
        {
            name: 'transfer';
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthorityPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'lightSystemProgram';
                    isMut: false;
                    isSigner: false;
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
                    name: 'selfProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'tokenPoolPda';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'compressOrDecompressTokenAccount';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'tokenProgram';
                    isMut: false;
                    isSigner: false;
                    isOptional: true;
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
            name: 'approve';
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthorityPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'lightSystemProgram';
                    isMut: false;
                    isSigner: false;
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
                    name: 'selfProgram';
                    isMut: false;
                    isSigner: false;
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
            name: 'revoke';
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthorityPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'lightSystemProgram';
                    isMut: false;
                    isSigner: false;
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
                    name: 'selfProgram';
                    isMut: false;
                    isSigner: false;
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
            name: 'freeze';
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthorityPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'lightSystemProgram';
                    isMut: false;
                    isSigner: false;
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
                    name: 'selfProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'mint';
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
            name: 'thaw';
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthorityPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'lightSystemProgram';
                    isMut: false;
                    isSigner: false;
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
                    name: 'selfProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'mint';
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
            name: 'burn';
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthorityPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'lightSystemProgram';
                    isMut: false;
                    isSigner: false;
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
                    name: 'selfProgram';
                    isMut: false;
                    isSigner: false;
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
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthorityPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'lightSystemProgram';
                    isMut: false;
                    isSigner: false;
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
                    name: 'selfProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'tokenPoolPda';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'compressOrDecompressTokenAccount';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'tokenProgram';
                    isMut: false;
                    isSigner: false;
                    isOptional: true;
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
                        defined: 'CompressedTokenInstructionDataTransfer';
                    };
                },
                {
                    name: 'inputs2';
                    type: {
                        defined: 'TokenData';
                    };
                },
            ];
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
            name: 'AccountState';
            type: {
                kind: 'enum';
                variants: [
                    {
                        name: 'Initialized';
                    },
                    {
                        name: 'Frozen';
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
            name: 'CompressedTokenInstructionDataTransfer';
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
                        name: 'mint';
                        type: 'publicKey';
                    },
                    {
                        name: 'delegatedTransfer';
                        docs: [
                            'If the signer is a delegate, the delegate index is index 0 of remaining accounts.',
                            'owner = Some(owner) is the owner of the token account.',
                            'Is set if the signer is delegate',
                        ];
                        type: {
                            option: {
                                defined: 'DelegatedTransfer';
                            };
                        };
                    },
                    {
                        name: 'inputTokenDataWithContext';
                        type: {
                            vec: {
                                defined: 'InputTokenDataWithContext';
                            };
                        };
                    },
                    {
                        name: 'outputCompressedAccounts';
                        type: {
                            vec: {
                                defined: 'PackedTokenTransferOutputData';
                            };
                        };
                    },
                    {
                        name: 'isCompress';
                        type: 'bool';
                    },
                    {
                        name: 'compressOrDecompressAmount';
                        type: {
                            option: 'u64';
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
            name: 'DelegatedTransfer';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'owner';
                        type: 'publicKey';
                    },
                    {
                        name: 'delegateChangeAccountIndex';
                        type: 'u8';
                    },
                ];
            };
        },
        {
            name: 'InputTokenDataWithContext';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'amount';
                        type: 'u64';
                    },
                    {
                        name: 'delegateIndex';
                        type: {
                            option: 'u8';
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
                        type: 'u16';
                    },
                    {
                        name: 'lamports';
                        type: {
                            option: 'u64';
                        };
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
            name: 'PackedTokenTransferOutputData';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'owner';
                        type: 'publicKey';
                    },
                    {
                        name: 'amount';
                        type: 'u64';
                    },
                    {
                        name: 'lamports';
                        type: {
                            option: 'u64';
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
            name: 'TokenData';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'mint';
                        docs: ['The mint associated with this account'];
                        type: 'publicKey';
                    },
                    {
                        name: 'owner';
                        docs: ['The owner of this account.'];
                        type: 'publicKey';
                    },
                    {
                        name: 'amount';
                        docs: ['The amount of tokens this account holds.'];
                        type: 'u64';
                    },
                    {
                        name: 'delegate';
                        docs: [
                            'If `delegate` is `Some` then `delegated_amount` represents',
                            'the amount authorized by the delegate',
                        ];
                        type: {
                            option: 'publicKey';
                        };
                    },
                    {
                        name: 'state';
                        docs: ["The account's state"];
                        type: {
                            defined: 'AccountState';
                        };
                    },
                ];
            };
        },
    ];
    errors: [
        {
            code: 6000;
            name: 'SignerCheckFailed';
            msg: 'Signer check failed';
        },
        {
            code: 6001;
            name: 'CreateTransferInstructionFailed';
            msg: 'Create transfer instruction failed';
        },
        {
            code: 6002;
            name: 'AccountNotFound';
            msg: 'Account not found';
        },
        {
            code: 6003;
            name: 'SerializationError';
            msg: 'Serialization error';
        },
    ];
};
export const IDL: LightCompressedToken = {
    version: '0.4.1',
    name: 'light_compressed_token',
    instructions: [
        {
            name: 'createTokenPool',
            docs: [
                'This instruction expects a mint account to be created in a separate',
                'token program instruction with token authority as mint authority. This',
                'instruction creates a token pool account for that mint owned by token',
                'authority.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'tokenPoolPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'mint',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'tokenProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'cpiAuthorityPda',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [],
        },
        {
            name: 'mintTo',
            docs: [
                'Mints tokens from an spl token mint to a list of compressed accounts.',
                'Minted tokens are transferred to a pool account owned by the compressed',
                'token program. The instruction creates one compressed output account for',
                'every amount and pubkey input pair one output compressed account.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthorityPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'mint',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'tokenPoolPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'tokenProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'lightSystemProgram',
                    isMut: false,
                    isSigner: false,
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
                    name: 'merkleTree',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'selfProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'publicKeys',
                    type: {
                        vec: 'publicKey',
                    },
                },
                {
                    name: 'amounts',
                    type: {
                        vec: 'u64',
                    },
                },
            ],
        },
        {
            name: 'transfer',
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthorityPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'lightSystemProgram',
                    isMut: false,
                    isSigner: false,
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
                    name: 'selfProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'tokenPoolPda',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'compressOrDecompressTokenAccount',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'tokenProgram',
                    isMut: false,
                    isSigner: false,
                    isOptional: true,
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
            name: 'approve',
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthorityPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'lightSystemProgram',
                    isMut: false,
                    isSigner: false,
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
                    name: 'selfProgram',
                    isMut: false,
                    isSigner: false,
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
            name: 'revoke',
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthorityPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'lightSystemProgram',
                    isMut: false,
                    isSigner: false,
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
                    name: 'selfProgram',
                    isMut: false,
                    isSigner: false,
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
            name: 'freeze',
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthorityPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'lightSystemProgram',
                    isMut: false,
                    isSigner: false,
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
                    name: 'selfProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'mint',
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
            name: 'thaw',
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthorityPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'lightSystemProgram',
                    isMut: false,
                    isSigner: false,
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
                    name: 'selfProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'mint',
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
            name: 'burn',
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthorityPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'lightSystemProgram',
                    isMut: false,
                    isSigner: false,
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
                    name: 'selfProgram',
                    isMut: false,
                    isSigner: false,
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
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthorityPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'lightSystemProgram',
                    isMut: false,
                    isSigner: false,
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
                    name: 'selfProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'tokenPoolPda',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'compressOrDecompressTokenAccount',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'tokenProgram',
                    isMut: false,
                    isSigner: false,
                    isOptional: true,
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
                        defined: 'CompressedTokenInstructionDataTransfer',
                    },
                },
                {
                    name: 'inputs2',
                    type: {
                        defined: 'TokenData',
                    },
                },
            ],
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
            name: 'AccountState',
            type: {
                kind: 'enum',
                variants: [
                    {
                        name: 'Initialized',
                    },
                    {
                        name: 'Frozen',
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
            name: 'CompressedTokenInstructionDataTransfer',
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
                        name: 'mint',
                        type: 'publicKey',
                    },
                    {
                        name: 'delegatedTransfer',
                        docs: [
                            'If the signer is a delegate, the delegate index is index 0 of remaining accounts.',
                            'owner = Some(owner) is the owner of the token account.',
                            'Is set if the signer is delegate',
                        ],
                        type: {
                            option: {
                                defined: 'DelegatedTransfer',
                            },
                        },
                    },
                    {
                        name: 'inputTokenDataWithContext',
                        type: {
                            vec: {
                                defined: 'InputTokenDataWithContext',
                            },
                        },
                    },
                    {
                        name: 'outputCompressedAccounts',
                        type: {
                            vec: {
                                defined: 'PackedTokenTransferOutputData',
                            },
                        },
                    },
                    {
                        name: 'isCompress',
                        type: 'bool',
                    },
                    {
                        name: 'compressOrDecompressAmount',
                        type: {
                            option: 'u64',
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
            name: 'DelegatedTransfer',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'owner',
                        type: 'publicKey',
                    },
                    {
                        name: 'delegateChangeAccountIndex',
                        type: 'u8',
                    },
                ],
            },
        },
        {
            name: 'InputTokenDataWithContext',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'amount',
                        type: 'u64',
                    },
                    {
                        name: 'delegateIndex',
                        type: {
                            option: 'u8',
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
                        type: 'u16',
                    },
                    {
                        name: 'lamports',
                        type: {
                            option: 'u64',
                        },
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
            name: 'PackedTokenTransferOutputData',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'owner',
                        type: 'publicKey',
                    },
                    {
                        name: 'amount',
                        type: 'u64',
                    },
                    {
                        name: 'lamports',
                        type: {
                            option: 'u64',
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
            name: 'TokenData',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'mint',
                        docs: ['The mint associated with this account'],
                        type: 'publicKey',
                    },
                    {
                        name: 'owner',
                        docs: ['The owner of this account.'],
                        type: 'publicKey',
                    },
                    {
                        name: 'amount',
                        docs: ['The amount of tokens this account holds.'],
                        type: 'u64',
                    },
                    {
                        name: 'delegate',
                        docs: [
                            'If `delegate` is `Some` then `delegated_amount` represents',
                            'the amount authorized by the delegate',
                        ],
                        type: {
                            option: 'publicKey',
                        },
                    },
                    {
                        name: 'state',
                        docs: ["The account's state"],
                        type: {
                            defined: 'AccountState',
                        },
                    },
                ],
            },
        },
    ],
    errors: [
        {
            code: 6000,
            name: 'SignerCheckFailed',
            msg: 'Signer check failed',
        },
        {
            code: 6001,
            name: 'CreateTransferInstructionFailed',
            msg: 'Create transfer instruction failed',
        },
        {
            code: 6002,
            name: 'AccountNotFound',
            msg: 'Account not found',
        },
        {
            code: 6003,
            name: 'SerializationError',
            msg: 'Serialization error',
        },
    ],
};
