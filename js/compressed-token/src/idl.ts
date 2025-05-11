export type LightCompressedToken = {
    version: '1.2.0';
    name: 'light_compressed_token';
    instructions: [
        {
            name: 'createTokenPool';
            docs: [
                'This instruction creates a token pool for a given mint. Every spl mint',
                'can have one token pool. When a token is compressed the tokens are',
                'transferrred to the token pool, and their compressed equivalent is',
                'minted into a Merkle tree.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: ['UNCHECKED: only pays fees.'];
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
            name: 'addTokenPool';
            docs: [
                'This instruction creates an additional token pool for a given mint.',
                'The maximum number of token pools per mint is 5.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: ['UNCHECKED: only pays fees.'];
                },
                {
                    name: 'tokenPoolPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'existingTokenPoolPda';
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
            args: [
                {
                    name: 'tokenPoolIndex';
                    type: 'u8';
                },
            ];
        },
        {
            name: 'mintTo';
            docs: [
                'Mints tokens from an spl token mint to a list of compressed accounts.',
                'Minted tokens are transferred to a pool account owned by the compressed',
                'token program. The instruction creates one compressed output account for',
                'every amount and pubkey input pair. A constant amount of lamports can be',
                'transferred to each output account to enable. A use case to add lamports',
                'to a compressed token account is to prevent spam. This is the only way',
                'to add lamports to a compressed token account.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: ['UNCHECKED: only pays fees.'];
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
                    docs: ['programs'];
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
                {
                    name: 'solPoolPda';
                    isMut: true;
                    isSigner: false;
                    isOptional: true;
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
                {
                    name: 'lamports';
                    type: {
                        option: 'u64';
                    };
                },
            ];
        },
        {
            name: 'compressSplTokenAccount';
            docs: [
                'Compresses the balance of an spl token account sub an optional remaining',
                'amount. This instruction does not close the spl token account. To close',
                'the account bundle a close spl account instruction in your transaction.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: ['UNCHECKED: only pays fees.'];
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
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
                    docs: ['this program is the signer of the cpi.'];
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
                    name: 'owner';
                    type: 'publicKey';
                },
                {
                    name: 'remainingAmount';
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
        },
        {
            name: 'transfer';
            docs: [
                'Transfers compressed tokens from one account to another. All accounts',
                'must be of the same mint. Additional spl tokens can be compressed or',
                'decompressed. In one transaction only compression or decompression is',
                'possible. Lamports can be transferred alongside tokens. If output token',
                'accounts specify less lamports than inputs the remaining lamports are',
                'transferred to an output compressed account. Signer must be owner or',
                'delegate. If a delegated token account is transferred the delegate is',
                'not preserved.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: ['UNCHECKED: only pays fees.'];
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
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
                    docs: ['this program is the signer of the cpi.'];
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
            docs: [
                'Delegates an amount to a delegate. A compressed token account is either',
                'completely delegated or not. Prior delegates are not preserved. Cannot',
                'be called by a delegate.',
                'The instruction creates two output accounts:',
                '1. one account with delegated amount',
                '2. one account with remaining(change) amount',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: ['UNCHECKED: only pays fees.'];
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
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
                    docs: ['this program is the signer of the cpi.'];
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
            docs: [
                'Revokes a delegation. The instruction merges all inputs into one output',
                'account. Cannot be called by a delegate. Delegates are not preserved.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: ['UNCHECKED: only pays fees.'];
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
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
                    docs: ['this program is the signer of the cpi.'];
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
            docs: [
                'Freezes compressed token accounts. Inputs must not be frozen. Creates as',
                'many outputs as inputs. Balances and delegates are preserved.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: ['UNCHECKED: only pays fees.'];
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
                    docs: ['that this program is the signer of the cpi.'];
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
            docs: [
                'Thaws frozen compressed token accounts. Inputs must be frozen. Creates',
                'as many outputs as inputs. Balances and delegates are preserved.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: ['UNCHECKED: only pays fees.'];
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
                    docs: ['that this program is the signer of the cpi.'];
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
            docs: [
                'Burns compressed tokens and spl tokens from the pool account. Delegates',
                'can burn tokens. The output compressed token account remains delegated.',
                'Creates one output compressed token account.',
            ];
            accounts: [
                {
                    name: 'feePayer';
                    isMut: true;
                    isSigner: true;
                    docs: ['UNCHECKED: only pays fees.'];
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
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
                    docs: ['UNCHECKED: only pays fees.'];
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
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
                    docs: ['this program is the signer of the cpi.'];
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
                            'Is required if the signer is delegate,',
                            '-> delegate is authority account,',
                            'owner = Some(owner) is the owner of the token account.',
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
                    {
                        name: 'lamportsChangeAccountMerkleTreeIndex';
                        type: {
                            option: 'u8';
                        };
                    },
                ];
            };
        },
        {
            name: 'CompressedTokenInstructionDataRevoke';
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
                        name: 'inputTokenDataWithContext';
                        type: {
                            vec: {
                                defined: 'InputTokenDataWithContext';
                            };
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
                    {
                        name: 'outputAccountMerkleTreeIndex';
                        type: 'u8';
                    },
                ];
            };
        },
        {
            name: 'CompressedTokenInstructionDataApprove';
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
                        name: 'inputTokenDataWithContext';
                        type: {
                            vec: {
                                defined: 'InputTokenDataWithContext';
                            };
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
                    {
                        name: 'delegate';
                        type: 'publicKey';
                    },
                    {
                        name: 'delegatedAmount';
                        type: 'u64';
                    },
                    {
                        name: 'delegateMerkleTreeIndex';
                        type: 'u8';
                    },
                    {
                        name: 'changeAccountMerkleTreeIndex';
                        type: 'u8';
                    },
                    {
                        name: 'delegateLamports';
                        type: {
                            option: 'u64';
                        };
                    },
                ];
            };
        },
        {
            name: 'DelegatedTransfer';
            docs: [
                'Struct to provide the owner when the delegate is signer of the transaction.',
            ];
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'owner';
                        type: 'publicKey';
                    },
                    {
                        name: 'delegateChangeAccountIndex';
                        docs: [
                            'Index of change compressed account in output compressed accounts. In',
                            "case that the delegate didn't spend the complete delegated compressed",
                            'account balance the change compressed account will be delegated to her',
                            'as well.',
                        ];
                        type: {
                            option: 'u8';
                        };
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
                    {
                        name: 'tlv';
                        docs: [
                            'Placeholder for TokenExtension tlv data (unimplemented)',
                        ];
                        type: {
                            option: 'bytes';
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
            name: 'PackedMerkleContext';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'merkleTreePubkeyIndex';
                        type: 'u8';
                    },
                    {
                        name: 'queuePubkeyIndex';
                        type: 'u8';
                    },
                    {
                        name: 'leafIndex';
                        type: 'u32';
                    },
                    {
                        name: 'proveByIndex';
                        type: 'bool';
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
                    {
                        name: 'tlv';
                        docs: [
                            'Placeholder for TokenExtension tlv data (unimplemented)',
                        ];
                        type: {
                            option: 'bytes';
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
                    {
                        name: 'tlv';
                        docs: [
                            'Placeholder for TokenExtension tlv data (unimplemented)',
                        ];
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
            name: 'PublicKeyAmountMissmatch';
            msg: 'public keys and amounts must be of same length';
        },
        {
            code: 6001;
            name: 'ComputeInputSumFailed';
            msg: 'ComputeInputSumFailed';
        },
        {
            code: 6002;
            name: 'ComputeOutputSumFailed';
            msg: 'ComputeOutputSumFailed';
        },
        {
            code: 6003;
            name: 'ComputeCompressSumFailed';
            msg: 'ComputeCompressSumFailed';
        },
        {
            code: 6004;
            name: 'ComputeDecompressSumFailed';
            msg: 'ComputeDecompressSumFailed';
        },
        {
            code: 6005;
            name: 'SumCheckFailed';
            msg: 'SumCheckFailed';
        },
        {
            code: 6006;
            name: 'DecompressRecipientUndefinedForDecompress';
            msg: 'DecompressRecipientUndefinedForDecompress';
        },
        {
            code: 6007;
            name: 'CompressedPdaUndefinedForDecompress';
            msg: 'CompressedPdaUndefinedForDecompress';
        },
        {
            code: 6008;
            name: 'DeCompressAmountUndefinedForDecompress';
            msg: 'DeCompressAmountUndefinedForDecompress';
        },
        {
            code: 6009;
            name: 'CompressedPdaUndefinedForCompress';
            msg: 'CompressedPdaUndefinedForCompress';
        },
        {
            code: 6010;
            name: 'DeCompressAmountUndefinedForCompress';
            msg: 'DeCompressAmountUndefinedForCompress';
        },
        {
            code: 6011;
            name: 'DelegateSignerCheckFailed';
            msg: 'DelegateSignerCheckFailed';
        },
        {
            code: 6012;
            name: 'MintTooLarge';
            msg: 'Minted amount greater than u64::MAX';
        },
        {
            code: 6013;
            name: 'SplTokenSupplyMismatch';
            msg: 'SplTokenSupplyMismatch';
        },
        {
            code: 6014;
            name: 'HeapMemoryCheckFailed';
            msg: 'HeapMemoryCheckFailed';
        },
        {
            code: 6015;
            name: 'InstructionNotCallable';
            msg: 'The instruction is not callable';
        },
        {
            code: 6016;
            name: 'ArithmeticUnderflow';
            msg: 'ArithmeticUnderflow';
        },
        {
            code: 6017;
            name: 'HashToFieldError';
            msg: 'HashToFieldError';
        },
        {
            code: 6018;
            name: 'InvalidAuthorityMint';
            msg: 'Expected the authority to be also a mint authority';
        },
        {
            code: 6019;
            name: 'InvalidFreezeAuthority';
            msg: 'Provided authority is not the freeze authority';
        },
        {
            code: 6020;
            name: 'InvalidDelegateIndex';
        },
        {
            code: 6021;
            name: 'TokenPoolPdaUndefined';
        },
        {
            code: 6022;
            name: 'IsTokenPoolPda';
            msg: 'Compress or decompress recipient is the same account as the token pool pda.';
        },
        {
            code: 6023;
            name: 'InvalidTokenPoolPda';
        },
        {
            code: 6024;
            name: 'NoInputTokenAccountsProvided';
        },
        {
            code: 6025;
            name: 'NoInputsProvided';
        },
        {
            code: 6026;
            name: 'MintHasNoFreezeAuthority';
        },
        {
            code: 6027;
            name: 'MintWithInvalidExtension';
        },
        {
            code: 6028;
            name: 'InsufficientTokenAccountBalance';
            msg: 'The token account balance is less than the remaining amount.';
        },
        {
            code: 6029;
            name: 'InvalidTokenPoolBump';
            msg: 'Max number of token pools reached.';
        },
        {
            code: 6030;
            name: 'FailedToDecompress';
        },
        {
            code: 6031;
            name: 'FailedToBurnSplTokensFromTokenPool';
        },
        {
            code: 6032;
            name: 'NoMatchingBumpFound';
        },
    ];
};
export const IDL: LightCompressedToken = {
    version: '1.2.0',
    name: 'light_compressed_token',
    instructions: [
        {
            name: 'createTokenPool',
            docs: [
                'This instruction creates a token pool for a given mint. Every spl mint',
                'can have one token pool. When a token is compressed the tokens are',
                'transferrred to the token pool, and their compressed equivalent is',
                'minted into a Merkle tree.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: ['UNCHECKED: only pays fees.'],
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
            name: 'addTokenPool',
            docs: [
                'This instruction creates an additional token pool for a given mint.',
                'The maximum number of token pools per mint is 5.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: ['UNCHECKED: only pays fees.'],
                },
                {
                    name: 'tokenPoolPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'existingTokenPoolPda',
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
            args: [
                {
                    name: 'tokenPoolIndex',
                    type: 'u8',
                },
            ],
        },
        {
            name: 'mintTo',
            docs: [
                'Mints tokens from an spl token mint to a list of compressed accounts.',
                'Minted tokens are transferred to a pool account owned by the compressed',
                'token program. The instruction creates one compressed output account for',
                'every amount and pubkey input pair. A constant amount of lamports can be',
                'transferred to each output account to enable. A use case to add lamports',
                'to a compressed token account is to prevent spam. This is the only way',
                'to add lamports to a compressed token account.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: ['UNCHECKED: only pays fees.'],
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
                    docs: ['programs'],
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
                {
                    name: 'solPoolPda',
                    isMut: true,
                    isSigner: false,
                    isOptional: true,
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
                {
                    name: 'lamports',
                    type: {
                        option: 'u64',
                    },
                },
            ],
        },
        {
            name: 'compressSplTokenAccount',
            docs: [
                'Compresses the balance of an spl token account sub an optional remaining',
                'amount. This instruction does not close the spl token account. To close',
                'the account bundle a close spl account instruction in your transaction.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: ['UNCHECKED: only pays fees.'],
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ],
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
                    docs: ['this program is the signer of the cpi.'],
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
                    name: 'owner',
                    type: 'publicKey',
                },
                {
                    name: 'remainingAmount',
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
        {
            name: 'transfer',
            docs: [
                'Transfers compressed tokens from one account to another. All accounts',
                'must be of the same mint. Additional spl tokens can be compressed or',
                'decompressed. In one transaction only compression or decompression is',
                'possible. Lamports can be transferred alongside tokens. If output token',
                'accounts specify less lamports than inputs the remaining lamports are',
                'transferred to an output compressed account. Signer must be owner or',
                'delegate. If a delegated token account is transferred the delegate is',
                'not preserved.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: ['UNCHECKED: only pays fees.'],
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ],
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
                    docs: ['this program is the signer of the cpi.'],
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
            docs: [
                'Delegates an amount to a delegate. A compressed token account is either',
                'completely delegated or not. Prior delegates are not preserved. Cannot',
                'be called by a delegate.',
                'The instruction creates two output accounts:',
                '1. one account with delegated amount',
                '2. one account with remaining(change) amount',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: ['UNCHECKED: only pays fees.'],
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ],
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
                    docs: ['this program is the signer of the cpi.'],
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
            docs: [
                'Revokes a delegation. The instruction merges all inputs into one output',
                'account. Cannot be called by a delegate. Delegates are not preserved.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: ['UNCHECKED: only pays fees.'],
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ],
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
                    docs: ['this program is the signer of the cpi.'],
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
            docs: [
                'Freezes compressed token accounts. Inputs must not be frozen. Creates as',
                'many outputs as inputs. Balances and delegates are preserved.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: ['UNCHECKED: only pays fees.'],
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
                    docs: ['that this program is the signer of the cpi.'],
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
            docs: [
                'Thaws frozen compressed token accounts. Inputs must be frozen. Creates',
                'as many outputs as inputs. Balances and delegates are preserved.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: ['UNCHECKED: only pays fees.'],
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
                    docs: ['that this program is the signer of the cpi.'],
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
            docs: [
                'Burns compressed tokens and spl tokens from the pool account. Delegates',
                'can burn tokens. The output compressed token account remains delegated.',
                'Creates one output compressed token account.',
            ],
            accounts: [
                {
                    name: 'feePayer',
                    isMut: true,
                    isSigner: true,
                    docs: ['UNCHECKED: only pays fees.'],
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ],
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
                    docs: ['UNCHECKED: only pays fees.'],
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ],
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
                    docs: ['this program is the signer of the cpi.'],
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
                            'Is required if the signer is delegate,',
                            '-> delegate is authority account,',
                            'owner = Some(owner) is the owner of the token account.',
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
                    {
                        name: 'lamportsChangeAccountMerkleTreeIndex',
                        type: {
                            option: 'u8',
                        },
                    },
                ],
            },
        },
        {
            name: 'CompressedTokenInstructionDataRevoke',
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
                        name: 'inputTokenDataWithContext',
                        type: {
                            vec: {
                                defined: 'InputTokenDataWithContext',
                            },
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
                    {
                        name: 'outputAccountMerkleTreeIndex',
                        type: 'u8',
                    },
                ],
            },
        },
        {
            name: 'CompressedTokenInstructionDataApprove',
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
                        name: 'inputTokenDataWithContext',
                        type: {
                            vec: {
                                defined: 'InputTokenDataWithContext',
                            },
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
                    {
                        name: 'delegate',
                        type: 'publicKey',
                    },
                    {
                        name: 'delegatedAmount',
                        type: 'u64',
                    },
                    {
                        name: 'delegateMerkleTreeIndex',
                        type: 'u8',
                    },
                    {
                        name: 'changeAccountMerkleTreeIndex',
                        type: 'u8',
                    },
                    {
                        name: 'delegateLamports',
                        type: {
                            option: 'u64',
                        },
                    },
                ],
            },
        },
        {
            name: 'DelegatedTransfer',
            docs: [
                'Struct to provide the owner when the delegate is signer of the transaction.',
            ],
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'owner',
                        type: 'publicKey',
                    },
                    {
                        name: 'delegateChangeAccountIndex',
                        docs: [
                            'Index of change compressed account in output compressed accounts. In',
                            "case that the delegate didn't spend the complete delegated compressed",
                            'account balance the change compressed account will be delegated to her',
                            'as well.',
                        ],
                        type: {
                            option: 'u8',
                        },
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
                    {
                        name: 'tlv',
                        docs: [
                            'Placeholder for TokenExtension tlv data (unimplemented)',
                        ],
                        type: {
                            option: 'bytes',
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
                    {
                        name: 'readOnly',
                        docs: [
                            'Placeholder to mark accounts read-only unimplemented set to false.',
                        ],
                        type: 'bool',
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
                        name: 'queuePubkeyIndex',
                        type: 'u8',
                    },
                    {
                        name: 'leafIndex',
                        type: 'u32',
                    },
                    {
                        name: 'proveByIndex',
                        type: 'bool',
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
                    {
                        name: 'tlv',
                        docs: [
                            'Placeholder for TokenExtension tlv data (unimplemented)',
                        ],
                        type: {
                            option: 'bytes',
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
                    {
                        name: 'tlv',
                        docs: [
                            'Placeholder for TokenExtension tlv data (unimplemented)',
                        ],
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
            name: 'PublicKeyAmountMissmatch',
            msg: 'public keys and amounts must be of same length',
        },
        {
            code: 6001,
            name: 'ComputeInputSumFailed',
            msg: 'ComputeInputSumFailed',
        },
        {
            code: 6002,
            name: 'ComputeOutputSumFailed',
            msg: 'ComputeOutputSumFailed',
        },
        {
            code: 6003,
            name: 'ComputeCompressSumFailed',
            msg: 'ComputeCompressSumFailed',
        },
        {
            code: 6004,
            name: 'ComputeDecompressSumFailed',
            msg: 'ComputeDecompressSumFailed',
        },
        {
            code: 6005,
            name: 'SumCheckFailed',
            msg: 'SumCheckFailed',
        },
        {
            code: 6006,
            name: 'DecompressRecipientUndefinedForDecompress',
            msg: 'DecompressRecipientUndefinedForDecompress',
        },
        {
            code: 6007,
            name: 'CompressedPdaUndefinedForDecompress',
            msg: 'CompressedPdaUndefinedForDecompress',
        },
        {
            code: 6008,
            name: 'DeCompressAmountUndefinedForDecompress',
            msg: 'DeCompressAmountUndefinedForDecompress',
        },
        {
            code: 6009,
            name: 'CompressedPdaUndefinedForCompress',
            msg: 'CompressedPdaUndefinedForCompress',
        },
        {
            code: 6010,
            name: 'DeCompressAmountUndefinedForCompress',
            msg: 'DeCompressAmountUndefinedForCompress',
        },
        {
            code: 6011,
            name: 'DelegateSignerCheckFailed',
            msg: 'DelegateSignerCheckFailed',
        },
        {
            code: 6012,
            name: 'MintTooLarge',
            msg: 'Minted amount greater than u64::MAX',
        },
        {
            code: 6013,
            name: 'SplTokenSupplyMismatch',
            msg: 'SplTokenSupplyMismatch',
        },
        {
            code: 6014,
            name: 'HeapMemoryCheckFailed',
            msg: 'HeapMemoryCheckFailed',
        },
        {
            code: 6015,
            name: 'InstructionNotCallable',
            msg: 'The instruction is not callable',
        },
        {
            code: 6016,
            name: 'ArithmeticUnderflow',
            msg: 'ArithmeticUnderflow',
        },
        {
            code: 6017,
            name: 'HashToFieldError',
            msg: 'HashToFieldError',
        },
        {
            code: 6018,
            name: 'InvalidAuthorityMint',
            msg: 'Expected the authority to be also a mint authority',
        },
        {
            code: 6019,
            name: 'InvalidFreezeAuthority',
            msg: 'Provided authority is not the freeze authority',
        },
        {
            code: 6020,
            name: 'InvalidDelegateIndex',
        },
        {
            code: 6021,
            name: 'TokenPoolPdaUndefined',
        },
        {
            code: 6022,
            name: 'IsTokenPoolPda',
            msg: 'Compress or decompress recipient is the same account as the token pool pda.',
        },
        {
            code: 6023,
            name: 'InvalidTokenPoolPda',
        },
        {
            code: 6024,
            name: 'NoInputTokenAccountsProvided',
        },
        {
            code: 6025,
            name: 'NoInputsProvided',
        },
        {
            code: 6026,
            name: 'MintHasNoFreezeAuthority',
        },
        {
            code: 6027,
            name: 'MintWithInvalidExtension',
        },
        {
            code: 6028,
            name: 'InsufficientTokenAccountBalance',
            msg: 'The token account balance is less than the remaining amount.',
        },
        {
            code: 6029,
            name: 'InvalidTokenPoolBump',
            msg: 'Max number of token pools reached.',
        },
        {
            code: 6030,
            name: 'FailedToDecompress',
        },
        {
            code: 6031,
            name: 'FailedToBurnSplTokensFromTokenPool',
        },
        {
            code: 6032,
            name: 'NoMatchingBumpFound',
        },
    ],
};
