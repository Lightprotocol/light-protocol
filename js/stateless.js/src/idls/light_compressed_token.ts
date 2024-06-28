/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/light_compressed_token.json`.
 */
export type LightCompressedToken = {
    address: 'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN';
    metadata: {
        name: 'lightCompressedToken';
        version: '0.5.0';
        spec: '0.1.0';
        description: 'Generalized token compression on Solana';
        repository: 'https://github.com/Lightprotocol/light-protocol';
    };
    instructions: [
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
            discriminator: [69, 74, 217, 36, 115, 117, 97, 76];
            accounts: [
                {
                    name: 'feePayer';
                    docs: ['UNCHECKED: only pays fees.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
                    signer: true;
                },
                {
                    name: 'cpiAuthorityPda';
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
                    name: 'lightSystemProgram';
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
                {
                    name: 'registeredProgramPda';
                },
                {
                    name: 'noopProgram';
                },
                {
                    name: 'accountCompressionAuthority';
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'selfProgram';
                    docs: ['this program is the signer of the cpi.'];
                    address: 'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN';
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
            name: 'burn';
            docs: [
                'Burns compressed tokens and spl tokens from the pool account. Delegates',
                'can burn tokens. The output compressed token account remains delegated.',
                'Creates one output compressed token account.',
            ];
            discriminator: [116, 110, 29, 56, 107, 219, 42, 93];
            accounts: [
                {
                    name: 'feePayer';
                    docs: ['UNCHECKED: only pays fees.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
                    signer: true;
                },
                {
                    name: 'cpiAuthorityPda';
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
                    name: 'mint';
                    writable: true;
                },
                {
                    name: 'tokenPoolPda';
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [112, 111, 111, 108];
                            },
                            {
                                kind: 'account';
                                path: 'mint';
                            },
                        ];
                    };
                },
                {
                    name: 'tokenProgram';
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';
                },
                {
                    name: 'lightSystemProgram';
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
                {
                    name: 'registeredProgramPda';
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
                        program: {
                            kind: 'const';
                            value: [
                                238,
                                250,
                                35,
                                216,
                                163,
                                90,
                                82,
                                72,
                                167,
                                209,
                                196,
                                227,
                                210,
                                173,
                                89,
                                255,
                                142,
                                20,
                                199,
                                150,
                                144,
                                215,
                                61,
                                164,
                                34,
                                47,
                                181,
                                228,
                                226,
                                153,
                                208,
                                17,
                            ];
                        };
                    };
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'selfProgram';
                    address: 'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN';
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
            name: 'createTokenPool';
            docs: [
                'This instruction creates a token pool for a given mint. Every spl mint',
                'can have one token pool. When a token is compressed the tokens are',
                'transferrred to the token pool, and their compressed equivalent is',
                'minted into a Merkle tree.',
            ];
            discriminator: [23, 169, 27, 122, 147, 169, 209, 152];
            accounts: [
                {
                    name: 'feePayer';
                    docs: ['UNCHECKED: only pays fees.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'tokenPoolPda';
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [112, 111, 111, 108];
                            },
                            {
                                kind: 'account';
                                path: 'mint';
                            },
                        ];
                    };
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
                {
                    name: 'mint';
                    writable: true;
                },
                {
                    name: 'tokenProgram';
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';
                },
                {
                    name: 'cpiAuthorityPda';
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
            ];
            args: [];
        },
        {
            name: 'freeze';
            docs: [
                'Freezes compressed token accounts. Inputs must not be frozen. Creates as',
                'many outputs as inputs. Balances and delegates are preserved.',
            ];
            discriminator: [255, 91, 207, 84, 251, 194, 254, 63];
            accounts: [
                {
                    name: 'feePayer';
                    docs: ['UNCHECKED: only pays fees.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpiAuthorityPda';
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
                    name: 'lightSystemProgram';
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
                {
                    name: 'registeredProgramPda';
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
                        program: {
                            kind: 'const';
                            value: [
                                238,
                                250,
                                35,
                                216,
                                163,
                                90,
                                82,
                                72,
                                167,
                                209,
                                196,
                                227,
                                210,
                                173,
                                89,
                                255,
                                142,
                                20,
                                199,
                                150,
                                144,
                                215,
                                61,
                                164,
                                34,
                                47,
                                181,
                                228,
                                226,
                                153,
                                208,
                                17,
                            ];
                        };
                    };
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'selfProgram';
                    docs: ['that this program is the signer of the cpi.'];
                    address: 'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN';
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
                {
                    name: 'mint';
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
            discriminator: [241, 34, 48, 186, 37, 179, 123, 192];
            accounts: [
                {
                    name: 'feePayer';
                    docs: ['UNCHECKED: only pays fees.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpiAuthorityPda';
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
                    name: 'mint';
                    writable: true;
                },
                {
                    name: 'tokenPoolPda';
                    docs: [
                        'account to a token account of a different mint will fail',
                    ];
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [112, 111, 111, 108];
                            },
                            {
                                kind: 'account';
                                path: 'mint';
                            },
                        ];
                    };
                },
                {
                    name: 'tokenProgram';
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';
                },
                {
                    name: 'lightSystemProgram';
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
                {
                    name: 'registeredProgramPda';
                },
                {
                    name: 'noopProgram';
                    docs: ['programs'];
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
                        program: {
                            kind: 'const';
                            value: [
                                238,
                                250,
                                35,
                                216,
                                163,
                                90,
                                82,
                                72,
                                167,
                                209,
                                196,
                                227,
                                210,
                                173,
                                89,
                                255,
                                142,
                                20,
                                199,
                                150,
                                144,
                                215,
                                61,
                                164,
                                34,
                                47,
                                181,
                                228,
                                226,
                                153,
                                208,
                                17,
                            ];
                        };
                    };
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'merkleTree';
                    writable: true;
                },
                {
                    name: 'selfProgram';
                    address: 'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN';
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
                {
                    name: 'solPoolPda';
                    writable: true;
                    optional: true;
                },
            ];
            args: [
                {
                    name: 'publicKeys';
                    type: {
                        vec: 'pubkey';
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
            name: 'revoke';
            docs: [
                'Revokes a delegation. The instruction merges all inputs into one output',
                'account. Cannot be called by a delegate. Delegates are not preserved.',
            ];
            discriminator: [170, 23, 31, 34, 133, 173, 93, 242];
            accounts: [
                {
                    name: 'feePayer';
                    docs: ['UNCHECKED: only pays fees.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
                    signer: true;
                },
                {
                    name: 'cpiAuthorityPda';
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
                    name: 'lightSystemProgram';
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
                {
                    name: 'registeredProgramPda';
                },
                {
                    name: 'noopProgram';
                },
                {
                    name: 'accountCompressionAuthority';
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'selfProgram';
                    docs: ['this program is the signer of the cpi.'];
                    address: 'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN';
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
                    docs: ['UNCHECKED: only pays fees.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
                    signer: true;
                },
                {
                    name: 'cpiAuthorityPda';
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
                    name: 'lightSystemProgram';
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
                {
                    name: 'registeredProgramPda';
                },
                {
                    name: 'noopProgram';
                },
                {
                    name: 'accountCompressionAuthority';
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'selfProgram';
                    docs: ['this program is the signer of the cpi.'];
                    address: 'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN';
                },
                {
                    name: 'tokenPoolPda';
                    writable: true;
                    optional: true;
                },
                {
                    name: 'compressOrDecompressTokenAccount';
                    writable: true;
                    optional: true;
                },
                {
                    name: 'tokenProgram';
                    optional: true;
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';
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
                            name: 'compressedTokenInstructionDataTransfer';
                        };
                    };
                },
                {
                    name: 'inputs2';
                    type: {
                        defined: {
                            name: 'tokenData';
                        };
                    };
                },
            ];
        },
        {
            name: 'thaw';
            docs: [
                'Thaws frozen compressed token accounts. Inputs must be frozen. Creates',
                'as many outputs as inputs. Balances and delegates are preserved.',
            ];
            discriminator: [226, 249, 34, 57, 189, 21, 177, 101];
            accounts: [
                {
                    name: 'feePayer';
                    docs: ['UNCHECKED: only pays fees.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpiAuthorityPda';
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
                    name: 'lightSystemProgram';
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
                {
                    name: 'registeredProgramPda';
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
                        program: {
                            kind: 'const';
                            value: [
                                238,
                                250,
                                35,
                                216,
                                163,
                                90,
                                82,
                                72,
                                167,
                                209,
                                196,
                                227,
                                210,
                                173,
                                89,
                                255,
                                142,
                                20,
                                199,
                                150,
                                144,
                                215,
                                61,
                                164,
                                34,
                                47,
                                181,
                                228,
                                226,
                                153,
                                208,
                                17,
                            ];
                        };
                    };
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'selfProgram';
                    docs: ['that this program is the signer of the cpi.'];
                    address: 'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN';
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
                {
                    name: 'mint';
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
            discriminator: [163, 52, 200, 231, 140, 3, 69, 186];
            accounts: [
                {
                    name: 'feePayer';
                    docs: ['UNCHECKED: only pays fees.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    docs: [
                        'Authority is verified through proof since both owner and delegate',
                        'are included in the token data hash, which is a public input to the',
                        'validity proof.',
                    ];
                    signer: true;
                },
                {
                    name: 'cpiAuthorityPda';
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
                    name: 'lightSystemProgram';
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
                {
                    name: 'registeredProgramPda';
                },
                {
                    name: 'noopProgram';
                },
                {
                    name: 'accountCompressionAuthority';
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'selfProgram';
                    docs: ['this program is the signer of the cpi.'];
                    address: 'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN';
                },
                {
                    name: 'tokenPoolPda';
                    writable: true;
                    optional: true;
                },
                {
                    name: 'compressOrDecompressTokenAccount';
                    writable: true;
                    optional: true;
                },
                {
                    name: 'tokenProgram';
                    optional: true;
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';
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
    ];
    errors: [
        {
            code: 6000;
            name: 'signerCheckFailed';
            msg: 'Signer check failed';
        },
        {
            code: 6001;
            name: 'createTransferInstructionFailed';
            msg: 'Create transfer instruction failed';
        },
        {
            code: 6002;
            name: 'accountNotFound';
            msg: 'Account not found';
        },
        {
            code: 6003;
            name: 'serializationError';
            msg: 'Serialization error';
        },
    ];
    types: [
        {
            name: 'accountState';
            repr: {
                kind: 'rust';
            };
            type: {
                kind: 'enum';
                variants: [
                    {
                        name: 'initialized';
                    },
                    {
                        name: 'frozen';
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
            name: 'compressedTokenInstructionDataTransfer';
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
                        name: 'mint';
                        type: 'pubkey';
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
                                defined: {
                                    name: 'delegatedTransfer';
                                };
                            };
                        };
                    },
                    {
                        name: 'inputTokenDataWithContext';
                        type: {
                            vec: {
                                defined: {
                                    name: 'inputTokenDataWithContext';
                                };
                            };
                        };
                    },
                    {
                        name: 'outputCompressedAccounts';
                        type: {
                            vec: {
                                defined: {
                                    name: 'packedTokenTransferOutputData';
                                };
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
                                defined: {
                                    name: 'compressedCpiContext';
                                };
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
            name: 'delegatedTransfer';
            docs: [
                'Struct to provide the owner when the delegate is signer of the transaction.',
            ];
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'owner';
                        type: 'pubkey';
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
            name: 'inputTokenDataWithContext';
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
                            defined: {
                                name: 'packedMerkleContext';
                            };
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
            name: 'packedTokenTransferOutputData';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'owner';
                        type: 'pubkey';
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
            name: 'tokenData';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'mint';
                        docs: ['The mint associated with this account'];
                        type: 'pubkey';
                    },
                    {
                        name: 'owner';
                        docs: ['The owner of this account.'];
                        type: 'pubkey';
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
                            option: 'pubkey';
                        };
                    },
                    {
                        name: 'state';
                        docs: ["The account's state"];
                        type: {
                            defined: {
                                name: 'accountState';
                            };
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
};
