export type LightCompressedToken = {
    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M';
    metadata: {
        name: 'light_compressed_token';
        version: '0.4.1';
        spec: '0.1.0';
        description: 'Generalized token compression on Solana';
        repository: 'https://github.com/Lightprotocol/light-protocol';
    };
    instructions: [
        {
            name: 'approve';
            discriminator: [69, 74, 217, 36, 115, 117, 97, 76];
            accounts: [
                {
                    name: 'fee_payer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpi_authority_pda';
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
                    name: 'light_system_program';
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL';
                },
                {
                    name: 'registered_program_pda';
                },
                {
                    name: 'noop_program';
                },
                {
                    name: 'account_compression_authority';
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
                        };
                    };
                },
                {
                    name: 'account_compression_program';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'self_program';
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M';
                },
                {
                    name: 'system_program';
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
            discriminator: [116, 110, 29, 56, 107, 219, 42, 93];
            accounts: [
                {
                    name: 'fee_payer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpi_authority_pda';
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
                    name: 'light_system_program';
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL';
                },
                {
                    name: 'registered_program_pda';
                },
                {
                    name: 'noop_program';
                },
                {
                    name: 'account_compression_authority';
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
                        };
                    };
                },
                {
                    name: 'account_compression_program';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'self_program';
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M';
                },
                {
                    name: 'system_program';
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
            name: 'create_token_pool';
            docs: [
                'This instruction expects a mint account to be created in a separate',
                'token program instruction with token authority as mint authority. This',
                'instruction creates a token pool account for that mint owned by token',
                'authority.',
            ];
            discriminator: [23, 169, 27, 122, 147, 169, 209, 152];
            accounts: [
                {
                    name: 'fee_payer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'token_pool_pda';
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
                    name: 'system_program';
                    address: '11111111111111111111111111111111';
                },
                {
                    name: 'mint';
                    writable: true;
                },
                {
                    name: 'token_program';
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';
                },
                {
                    name: 'cpi_authority_pda';
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
            discriminator: [255, 91, 207, 84, 251, 194, 254, 63];
            accounts: [
                {
                    name: 'fee_payer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpi_authority_pda';
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
                    name: 'light_system_program';
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL';
                },
                {
                    name: 'registered_program_pda';
                },
                {
                    name: 'noop_program';
                },
                {
                    name: 'account_compression_authority';
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
                        };
                    };
                },
                {
                    name: 'account_compression_program';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'self_program';
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M';
                },
                {
                    name: 'system_program';
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
            name: 'mint_to';
            docs: [
                'Mints tokens from an spl token mint to a list of compressed accounts.',
                'Minted tokens are transferred to a pool account owned by the compressed',
                'token program. The instruction creates one compressed output account for',
                'every amount and pubkey input pair one output compressed account.',
            ];
            discriminator: [241, 34, 48, 186, 37, 179, 123, 192];
            accounts: [
                {
                    name: 'fee_payer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpi_authority_pda';
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
                    name: 'token_pool_pda';
                    writable: true;
                },
                {
                    name: 'token_program';
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';
                },
                {
                    name: 'light_system_program';
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL';
                },
                {
                    name: 'registered_program_pda';
                },
                {
                    name: 'noop_program';
                },
                {
                    name: 'account_compression_authority';
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
                        };
                    };
                },
                {
                    name: 'account_compression_program';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'merkle_tree';
                    writable: true;
                },
                {
                    name: 'self_program';
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M';
                },
                {
                    name: 'system_program';
                    address: '11111111111111111111111111111111';
                },
            ];
            args: [
                {
                    name: 'public_keys';
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
            ];
        },
        {
            name: 'revoke';
            discriminator: [170, 23, 31, 34, 133, 173, 93, 242];
            accounts: [
                {
                    name: 'fee_payer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpi_authority_pda';
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
                    name: 'light_system_program';
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL';
                },
                {
                    name: 'registered_program_pda';
                },
                {
                    name: 'noop_program';
                },
                {
                    name: 'account_compression_authority';
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
                        };
                    };
                },
                {
                    name: 'account_compression_program';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'self_program';
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M';
                },
                {
                    name: 'system_program';
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
            name: 'thaw';
            discriminator: [226, 249, 34, 57, 189, 21, 177, 101];
            accounts: [
                {
                    name: 'fee_payer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpi_authority_pda';
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
                    name: 'light_system_program';
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL';
                },
                {
                    name: 'registered_program_pda';
                },
                {
                    name: 'noop_program';
                },
                {
                    name: 'account_compression_authority';
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
                        };
                    };
                },
                {
                    name: 'account_compression_program';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'self_program';
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M';
                },
                {
                    name: 'system_program';
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
            discriminator: [163, 52, 200, 231, 140, 3, 69, 186];
            accounts: [
                {
                    name: 'fee_payer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpi_authority_pda';
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
                    name: 'light_system_program';
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL';
                },
                {
                    name: 'registered_program_pda';
                },
                {
                    name: 'noop_program';
                },
                {
                    name: 'account_compression_authority';
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
                        };
                    };
                },
                {
                    name: 'account_compression_program';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'self_program';
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M';
                },
                {
                    name: 'token_pool_pda';
                    writable: true;
                    optional: true;
                },
                {
                    name: 'compress_or_decompress_token_account';
                    writable: true;
                    optional: true;
                },
                {
                    name: 'token_program';
                    optional: true;
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';
                },
                {
                    name: 'system_program';
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
    accounts: [
        {
            name: 'RegisteredProgram';
            discriminator: [31, 251, 180, 235, 3, 116, 50, 4];
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
            name: 'SignerCheckFailed';
            msg: 'SignerCheckFailed';
        },
        {
            code: 6002;
            name: 'ComputeInputSumFailed';
            msg: 'ComputeInputSumFailed';
        },
        {
            code: 6003;
            name: 'ComputeOutputSumFailed';
            msg: 'ComputeOutputSumFailed';
        },
        {
            code: 6004;
            name: 'ComputeCompressSumFailed';
            msg: 'ComputeCompressSumFailed';
        },
        {
            code: 6005;
            name: 'ComputeDecompressSumFailed';
            msg: 'ComputeDecompressSumFailed';
        },
        {
            code: 6006;
            name: 'SumCheckFailed';
            msg: 'SumCheckFailed';
        },
        {
            code: 6007;
            name: 'DecompressRecipientUndefinedForDecompress';
            msg: 'DecompressRecipientUndefinedForDecompress';
        },
        {
            code: 6008;
            name: 'CompressedPdaUndefinedForDecompress';
            msg: 'CompressedPdaUndefinedForDecompress';
        },
        {
            code: 6009;
            name: 'DeCompressAmountUndefinedForDecompress';
            msg: 'DeCompressAmountUndefinedForDecompress';
        },
        {
            code: 6010;
            name: 'CompressedPdaUndefinedForCompress';
            msg: 'CompressedPdaUndefinedForCompress';
        },
        {
            code: 6011;
            name: 'DeCompressAmountUndefinedForCompress';
            msg: 'DeCompressAmountUndefinedForCompress';
        },
        {
            code: 6012;
            name: 'DelegateUndefined';
            msg: 'DelegateUndefined while delegated amount is defined';
        },
        {
            code: 6013;
            name: 'DelegateSignerCheckFailed';
            msg: 'DelegateSignerCheckFailed';
        },
        {
            code: 6014;
            name: 'SplTokenSupplyMismatch';
            msg: 'SplTokenSupplyMismatch';
        },
        {
            code: 6015;
            name: 'HeapMemoryCheckFailed';
            msg: 'HeapMemoryCheckFailed';
        },
        {
            code: 6016;
            name: 'InstructionNotCallable';
            msg: 'The instruction is not callable';
        },
        {
            code: 6017;
            name: 'ArithmeticUnderflow';
            msg: 'ArithmeticUnderflow';
        },
        {
            code: 6018;
            name: 'InvalidDelegate';
            msg: 'InvalidDelegate';
        },
        {
            code: 6019;
            name: 'HashToFieldError';
            msg: 'HashToFieldError';
        },
        {
            code: 6020;
            name: 'InvalidMint';
            msg: 'InvalidMint';
        },
    ];
    types: [
        {
            name: 'AccessMetadata';
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
                        name: 'program_owner';
                        docs: [
                            'Delegate of the Merkle tree. This will be used for program owned Merkle trees.',
                        ];
                        type: 'pubkey';
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
                                    name: 'CompressedAccountData';
                                };
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
                        name: 'data_hash';
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
                        name: 'set_context';
                        docs: [
                            'Is set by the program that is invoking the CPI to signal that is should',
                            'set the cpi context.',
                        ];
                        type: 'bool';
                    },
                    {
                        name: 'first_set_context';
                        docs: [
                            'Is set to wipe the cpi context since someone could have set it before',
                            'with unrelated data.',
                        ];
                        type: 'bool';
                    },
                    {
                        name: 'cpi_context_account_index';
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
            name: 'CpiContextAccount';
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
                        name: 'fee_payer';
                        type: 'pubkey';
                    },
                    {
                        name: 'associated_merkle_tree';
                        type: 'pubkey';
                    },
                    {
                        name: 'context';
                        type: {
                            vec: {
                                defined: {
                                    name: 'InstructionDataInvokeCpi';
                                };
                            };
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
                                defined: {
                                    name: 'CompressedProof';
                                };
                            };
                        };
                    },
                    {
                        name: 'new_address_params';
                        type: {
                            vec: {
                                defined: {
                                    name: 'NewAddressParamsPacked';
                                };
                            };
                        };
                    },
                    {
                        name: 'input_compressed_accounts_with_merkle_context';
                        type: {
                            vec: {
                                defined: {
                                    name: 'PackedCompressedAccountWithMerkleContext';
                                };
                            };
                        };
                    },
                    {
                        name: 'output_compressed_accounts';
                        type: {
                            vec: {
                                defined: {
                                    name: 'OutputCompressedAccountWithPackedContext';
                                };
                            };
                        };
                    },
                    {
                        name: 'relay_fee';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'compress_or_decompress_lamports';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'is_compress';
                        type: 'bool';
                    },
                    {
                        name: 'signer_seeds';
                        type: {
                            vec: 'bytes';
                        };
                    },
                    {
                        name: 'cpi_context';
                        type: {
                            option: {
                                defined: {
                                    name: 'CompressedCpiContext';
                                };
                            };
                        };
                    },
                ];
            };
        },
        {
            name: 'MerkleTreeMetadata';
            serialization: 'bytemuck';
            repr: {
                kind: 'c';
            };
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'access_metadata';
                        type: {
                            defined: {
                                name: 'AccessMetadata';
                            };
                        };
                    },
                    {
                        name: 'rollover_metadata';
                        type: {
                            defined: {
                                name: 'RolloverMetadata';
                            };
                        };
                    },
                    {
                        name: 'associated_queue';
                        type: 'pubkey';
                    },
                    {
                        name: 'next_merkle_tree';
                        type: 'pubkey';
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
                        name: 'address_queue_account_index';
                        type: 'u8';
                    },
                    {
                        name: 'address_merkle_tree_account_index';
                        type: 'u8';
                    },
                    {
                        name: 'address_merkle_tree_root_index';
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
                        name: 'compressed_account';
                        type: {
                            defined: {
                                name: 'CompressedAccount';
                            };
                        };
                    },
                    {
                        name: 'merkle_tree_index';
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
                        name: 'compressed_account';
                        type: {
                            defined: {
                                name: 'CompressedAccount';
                            };
                        };
                    },
                    {
                        name: 'merkle_context';
                        type: {
                            defined: {
                                name: 'PackedMerkleContext';
                            };
                        };
                    },
                    {
                        name: 'root_index';
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
                        name: 'merkle_tree_pubkey_index';
                        type: 'u8';
                    },
                    {
                        name: 'nullifier_queue_pubkey_index';
                        type: 'u8';
                    },
                    {
                        name: 'leaf_index';
                        type: 'u32';
                    },
                ];
            };
        },
        {
            name: 'RegisteredProgram';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'registered_program_id';
                        type: 'pubkey';
                    },
                    {
                        name: 'group_authority_pda';
                        type: 'pubkey';
                    },
                ];
            };
        },
        {
            name: 'RolloverMetadata';
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
                        name: 'rollover_fee';
                        docs: [
                            'This fee is used for rent for the next account.',
                            'It accumulates in the account so that once the corresponding Merkle tree account is full it can be rolled over',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'rollover_threshold';
                        docs: [
                            'The threshold in percentage points when the account should be rolled over (95 corresponds to 95% filled).',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'network_fee';
                        docs: ['Tip for maintaining the account.'];
                        type: 'u64';
                    },
                    {
                        name: 'rolledover_slot';
                        docs: [
                            'The slot when the account was rolled over, a rolled over account should not be written to.',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'close_threshold';
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
            name: 'StateMerkleTreeAccount';
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
                                name: 'MerkleTreeMetadata';
                            };
                        };
                    },
                ];
            };
        },
    ];
};
export const IDL: LightCompressedToken = {
    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M',
    metadata: {
        name: 'light_compressed_token',
        version: '0.4.1',
        spec: '0.1.0',
        description: 'Generalized token compression on Solana',
        repository: 'https://github.com/Lightprotocol/light-protocol',
    },
    instructions: [
        {
            name: 'approve',
            discriminator: [69, 74, 217, 36, 115, 117, 97, 76],
            accounts: [
                {
                    name: 'fee_payer',
                    writable: true,
                    signer: true,
                },
                {
                    name: 'authority',
                    signer: true,
                },
                {
                    name: 'cpi_authority_pda',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                    },
                },
                {
                    name: 'light_system_program',
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL',
                },
                {
                    name: 'registered_program_pda',
                },
                {
                    name: 'noop_program',
                },
                {
                    name: 'account_compression_authority',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                        program: {
                            kind: 'const',
                            value: [
                                122, 247, 228, 253, 125, 235, 168, 178, 118,
                                247, 148, 132, 87, 69, 138, 249, 48, 38, 162,
                                157, 101, 94, 204, 195, 59, 23, 183, 138, 180,
                                125, 89, 223,
                            ],
                        },
                    },
                },
                {
                    name: 'account_compression_program',
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U',
                },
                {
                    name: 'self_program',
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M',
                },
                {
                    name: 'system_program',
                    address: '11111111111111111111111111111111',
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
            discriminator: [116, 110, 29, 56, 107, 219, 42, 93],
            accounts: [
                {
                    name: 'fee_payer',
                    writable: true,
                    signer: true,
                },
                {
                    name: 'authority',
                    signer: true,
                },
                {
                    name: 'cpi_authority_pda',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                    },
                },
                {
                    name: 'light_system_program',
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL',
                },
                {
                    name: 'registered_program_pda',
                },
                {
                    name: 'noop_program',
                },
                {
                    name: 'account_compression_authority',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                        program: {
                            kind: 'const',
                            value: [
                                122, 247, 228, 253, 125, 235, 168, 178, 118,
                                247, 148, 132, 87, 69, 138, 249, 48, 38, 162,
                                157, 101, 94, 204, 195, 59, 23, 183, 138, 180,
                                125, 89, 223,
                            ],
                        },
                    },
                },
                {
                    name: 'account_compression_program',
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U',
                },
                {
                    name: 'self_program',
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M',
                },
                {
                    name: 'system_program',
                    address: '11111111111111111111111111111111',
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
            name: 'create_token_pool',
            docs: [
                'This instruction expects a mint account to be created in a separate',
                'token program instruction with token authority as mint authority. This',
                'instruction creates a token pool account for that mint owned by token',
                'authority.',
            ],
            discriminator: [23, 169, 27, 122, 147, 169, 209, 152],
            accounts: [
                {
                    name: 'fee_payer',
                    writable: true,
                    signer: true,
                },
                {
                    name: 'token_pool_pda',
                    writable: true,
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [112, 111, 111, 108],
                            },
                            {
                                kind: 'account',
                                path: 'mint',
                            },
                        ],
                    },
                },
                {
                    name: 'system_program',
                    address: '11111111111111111111111111111111',
                },
                {
                    name: 'mint',
                    writable: true,
                },
                {
                    name: 'token_program',
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
                },
                {
                    name: 'cpi_authority_pda',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                    },
                },
            ],
            args: [],
        },
        {
            name: 'freeze',
            discriminator: [255, 91, 207, 84, 251, 194, 254, 63],
            accounts: [
                {
                    name: 'fee_payer',
                    writable: true,
                    signer: true,
                },
                {
                    name: 'authority',
                    signer: true,
                },
                {
                    name: 'cpi_authority_pda',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                    },
                },
                {
                    name: 'light_system_program',
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL',
                },
                {
                    name: 'registered_program_pda',
                },
                {
                    name: 'noop_program',
                },
                {
                    name: 'account_compression_authority',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                        program: {
                            kind: 'const',
                            value: [
                                122, 247, 228, 253, 125, 235, 168, 178, 118,
                                247, 148, 132, 87, 69, 138, 249, 48, 38, 162,
                                157, 101, 94, 204, 195, 59, 23, 183, 138, 180,
                                125, 89, 223,
                            ],
                        },
                    },
                },
                {
                    name: 'account_compression_program',
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U',
                },
                {
                    name: 'self_program',
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M',
                },
                {
                    name: 'system_program',
                    address: '11111111111111111111111111111111',
                },
                {
                    name: 'mint',
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
            name: 'mint_to',
            docs: [
                'Mints tokens from an spl token mint to a list of compressed accounts.',
                'Minted tokens are transferred to a pool account owned by the compressed',
                'token program. The instruction creates one compressed output account for',
                'every amount and pubkey input pair one output compressed account.',
            ],
            discriminator: [241, 34, 48, 186, 37, 179, 123, 192],
            accounts: [
                {
                    name: 'fee_payer',
                    writable: true,
                    signer: true,
                },
                {
                    name: 'authority',
                    signer: true,
                },
                {
                    name: 'cpi_authority_pda',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                    },
                },
                {
                    name: 'mint',
                    writable: true,
                },
                {
                    name: 'token_pool_pda',
                    writable: true,
                },
                {
                    name: 'token_program',
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
                },
                {
                    name: 'light_system_program',
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL',
                },
                {
                    name: 'registered_program_pda',
                },
                {
                    name: 'noop_program',
                },
                {
                    name: 'account_compression_authority',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                        program: {
                            kind: 'const',
                            value: [
                                122, 247, 228, 253, 125, 235, 168, 178, 118,
                                247, 148, 132, 87, 69, 138, 249, 48, 38, 162,
                                157, 101, 94, 204, 195, 59, 23, 183, 138, 180,
                                125, 89, 223,
                            ],
                        },
                    },
                },
                {
                    name: 'account_compression_program',
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U',
                },
                {
                    name: 'merkle_tree',
                    writable: true,
                },
                {
                    name: 'self_program',
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M',
                },
                {
                    name: 'system_program',
                    address: '11111111111111111111111111111111',
                },
            ],
            args: [
                {
                    name: 'public_keys',
                    type: {
                        vec: 'pubkey',
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
            name: 'revoke',
            discriminator: [170, 23, 31, 34, 133, 173, 93, 242],
            accounts: [
                {
                    name: 'fee_payer',
                    writable: true,
                    signer: true,
                },
                {
                    name: 'authority',
                    signer: true,
                },
                {
                    name: 'cpi_authority_pda',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                    },
                },
                {
                    name: 'light_system_program',
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL',
                },
                {
                    name: 'registered_program_pda',
                },
                {
                    name: 'noop_program',
                },
                {
                    name: 'account_compression_authority',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                        program: {
                            kind: 'const',
                            value: [
                                122, 247, 228, 253, 125, 235, 168, 178, 118,
                                247, 148, 132, 87, 69, 138, 249, 48, 38, 162,
                                157, 101, 94, 204, 195, 59, 23, 183, 138, 180,
                                125, 89, 223,
                            ],
                        },
                    },
                },
                {
                    name: 'account_compression_program',
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U',
                },
                {
                    name: 'self_program',
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M',
                },
                {
                    name: 'system_program',
                    address: '11111111111111111111111111111111',
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
            discriminator: [226, 249, 34, 57, 189, 21, 177, 101],
            accounts: [
                {
                    name: 'fee_payer',
                    writable: true,
                    signer: true,
                },
                {
                    name: 'authority',
                    signer: true,
                },
                {
                    name: 'cpi_authority_pda',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                    },
                },
                {
                    name: 'light_system_program',
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL',
                },
                {
                    name: 'registered_program_pda',
                },
                {
                    name: 'noop_program',
                },
                {
                    name: 'account_compression_authority',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                        program: {
                            kind: 'const',
                            value: [
                                122, 247, 228, 253, 125, 235, 168, 178, 118,
                                247, 148, 132, 87, 69, 138, 249, 48, 38, 162,
                                157, 101, 94, 204, 195, 59, 23, 183, 138, 180,
                                125, 89, 223,
                            ],
                        },
                    },
                },
                {
                    name: 'account_compression_program',
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U',
                },
                {
                    name: 'self_program',
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M',
                },
                {
                    name: 'system_program',
                    address: '11111111111111111111111111111111',
                },
                {
                    name: 'mint',
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
            name: 'transfer',
            discriminator: [163, 52, 200, 231, 140, 3, 69, 186],
            accounts: [
                {
                    name: 'fee_payer',
                    writable: true,
                    signer: true,
                },
                {
                    name: 'authority',
                    signer: true,
                },
                {
                    name: 'cpi_authority_pda',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                    },
                },
                {
                    name: 'light_system_program',
                    address: '9H1yjuq1gCLwQArXphX3aFKkeWZ7oK3i3C45HcfNGgdL',
                },
                {
                    name: 'registered_program_pda',
                },
                {
                    name: 'noop_program',
                },
                {
                    name: 'account_compression_authority',
                    pda: {
                        seeds: [
                            {
                                kind: 'const',
                                value: [
                                    99, 112, 105, 95, 97, 117, 116, 104, 111,
                                    114, 105, 116, 121,
                                ],
                            },
                        ],
                        program: {
                            kind: 'const',
                            value: [
                                122, 247, 228, 253, 125, 235, 168, 178, 118,
                                247, 148, 132, 87, 69, 138, 249, 48, 38, 162,
                                157, 101, 94, 204, 195, 59, 23, 183, 138, 180,
                                125, 89, 223,
                            ],
                        },
                    },
                },
                {
                    name: 'account_compression_program',
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U',
                },
                {
                    name: 'self_program',
                    address: '6kvxACq6SVLMiWbfsWACAFuZutrD2F1J9G8rb9CDao4M',
                },
                {
                    name: 'token_pool_pda',
                    writable: true,
                    optional: true,
                },
                {
                    name: 'compress_or_decompress_token_account',
                    writable: true,
                    optional: true,
                },
                {
                    name: 'token_program',
                    optional: true,
                    address: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
                },
                {
                    name: 'system_program',
                    address: '11111111111111111111111111111111',
                },
            ],
            args: [
                {
                    name: 'inputs',
                    type: 'bytes',
                },
            ],
        },
    ],
    accounts: [
        {
            name: 'RegisteredProgram',
            discriminator: [31, 251, 180, 235, 3, 116, 50, 4],
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
            name: 'SignerCheckFailed',
            msg: 'SignerCheckFailed',
        },
        {
            code: 6002,
            name: 'ComputeInputSumFailed',
            msg: 'ComputeInputSumFailed',
        },
        {
            code: 6003,
            name: 'ComputeOutputSumFailed',
            msg: 'ComputeOutputSumFailed',
        },
        {
            code: 6004,
            name: 'ComputeCompressSumFailed',
            msg: 'ComputeCompressSumFailed',
        },
        {
            code: 6005,
            name: 'ComputeDecompressSumFailed',
            msg: 'ComputeDecompressSumFailed',
        },
        {
            code: 6006,
            name: 'SumCheckFailed',
            msg: 'SumCheckFailed',
        },
        {
            code: 6007,
            name: 'DecompressRecipientUndefinedForDecompress',
            msg: 'DecompressRecipientUndefinedForDecompress',
        },
        {
            code: 6008,
            name: 'CompressedPdaUndefinedForDecompress',
            msg: 'CompressedPdaUndefinedForDecompress',
        },
        {
            code: 6009,
            name: 'DeCompressAmountUndefinedForDecompress',
            msg: 'DeCompressAmountUndefinedForDecompress',
        },
        {
            code: 6010,
            name: 'CompressedPdaUndefinedForCompress',
            msg: 'CompressedPdaUndefinedForCompress',
        },
        {
            code: 6011,
            name: 'DeCompressAmountUndefinedForCompress',
            msg: 'DeCompressAmountUndefinedForCompress',
        },
        {
            code: 6012,
            name: 'DelegateUndefined',
            msg: 'DelegateUndefined while delegated amount is defined',
        },
        {
            code: 6013,
            name: 'DelegateSignerCheckFailed',
            msg: 'DelegateSignerCheckFailed',
        },
        {
            code: 6014,
            name: 'SplTokenSupplyMismatch',
            msg: 'SplTokenSupplyMismatch',
        },
        {
            code: 6015,
            name: 'HeapMemoryCheckFailed',
            msg: 'HeapMemoryCheckFailed',
        },
        {
            code: 6016,
            name: 'InstructionNotCallable',
            msg: 'The instruction is not callable',
        },
        {
            code: 6017,
            name: 'ArithmeticUnderflow',
            msg: 'ArithmeticUnderflow',
        },
        {
            code: 6018,
            name: 'InvalidDelegate',
            msg: 'InvalidDelegate',
        },
        {
            code: 6019,
            name: 'HashToFieldError',
            msg: 'HashToFieldError',
        },
        {
            code: 6020,
            name: 'InvalidMint',
            msg: 'InvalidMint',
        },
    ],
    types: [
        {
            name: 'AccessMetadata',
            serialization: 'bytemuck',
            repr: {
                kind: 'c',
            },
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'owner',
                        docs: ['Owner of the Merkle tree.'],
                        type: 'pubkey',
                    },
                    {
                        name: 'program_owner',
                        docs: [
                            'Delegate of the Merkle tree. This will be used for program owned Merkle trees.',
                        ],
                        type: 'pubkey',
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
                        type: 'pubkey',
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
                                defined: {
                                    name: 'CompressedAccountData',
                                },
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
                        name: 'data_hash',
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
                        name: 'set_context',
                        docs: [
                            'Is set by the program that is invoking the CPI to signal that is should',
                            'set the cpi context.',
                        ],
                        type: 'bool',
                    },
                    {
                        name: 'first_set_context',
                        docs: [
                            'Is set to wipe the cpi context since someone could have set it before',
                            'with unrelated data.',
                        ],
                        type: 'bool',
                    },
                    {
                        name: 'cpi_context_account_index',
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
            name: 'CpiContextAccount',
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
                        name: 'fee_payer',
                        type: 'pubkey',
                    },
                    {
                        name: 'associated_merkle_tree',
                        type: 'pubkey',
                    },
                    {
                        name: 'context',
                        type: {
                            vec: {
                                defined: {
                                    name: 'InstructionDataInvokeCpi',
                                },
                            },
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
                                defined: {
                                    name: 'CompressedProof',
                                },
                            },
                        },
                    },
                    {
                        name: 'new_address_params',
                        type: {
                            vec: {
                                defined: {
                                    name: 'NewAddressParamsPacked',
                                },
                            },
                        },
                    },
                    {
                        name: 'input_compressed_accounts_with_merkle_context',
                        type: {
                            vec: {
                                defined: {
                                    name: 'PackedCompressedAccountWithMerkleContext',
                                },
                            },
                        },
                    },
                    {
                        name: 'output_compressed_accounts',
                        type: {
                            vec: {
                                defined: {
                                    name: 'OutputCompressedAccountWithPackedContext',
                                },
                            },
                        },
                    },
                    {
                        name: 'relay_fee',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'compress_or_decompress_lamports',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'is_compress',
                        type: 'bool',
                    },
                    {
                        name: 'signer_seeds',
                        type: {
                            vec: 'bytes',
                        },
                    },
                    {
                        name: 'cpi_context',
                        type: {
                            option: {
                                defined: {
                                    name: 'CompressedCpiContext',
                                },
                            },
                        },
                    },
                ],
            },
        },
        {
            name: 'MerkleTreeMetadata',
            serialization: 'bytemuck',
            repr: {
                kind: 'c',
            },
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'access_metadata',
                        type: {
                            defined: {
                                name: 'AccessMetadata',
                            },
                        },
                    },
                    {
                        name: 'rollover_metadata',
                        type: {
                            defined: {
                                name: 'RolloverMetadata',
                            },
                        },
                    },
                    {
                        name: 'associated_queue',
                        type: 'pubkey',
                    },
                    {
                        name: 'next_merkle_tree',
                        type: 'pubkey',
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
                        name: 'address_queue_account_index',
                        type: 'u8',
                    },
                    {
                        name: 'address_merkle_tree_account_index',
                        type: 'u8',
                    },
                    {
                        name: 'address_merkle_tree_root_index',
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
                        name: 'compressed_account',
                        type: {
                            defined: {
                                name: 'CompressedAccount',
                            },
                        },
                    },
                    {
                        name: 'merkle_tree_index',
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
                        name: 'compressed_account',
                        type: {
                            defined: {
                                name: 'CompressedAccount',
                            },
                        },
                    },
                    {
                        name: 'merkle_context',
                        type: {
                            defined: {
                                name: 'PackedMerkleContext',
                            },
                        },
                    },
                    {
                        name: 'root_index',
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
                        name: 'merkle_tree_pubkey_index',
                        type: 'u8',
                    },
                    {
                        name: 'nullifier_queue_pubkey_index',
                        type: 'u8',
                    },
                    {
                        name: 'leaf_index',
                        type: 'u32',
                    },
                ],
            },
        },
        {
            name: 'RegisteredProgram',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'registered_program_id',
                        type: 'pubkey',
                    },
                    {
                        name: 'group_authority_pda',
                        type: 'pubkey',
                    },
                ],
            },
        },
        {
            name: 'RolloverMetadata',
            serialization: 'bytemuck',
            repr: {
                kind: 'c',
            },
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'index',
                        docs: ['Unique index.'],
                        type: 'u64',
                    },
                    {
                        name: 'rollover_fee',
                        docs: [
                            'This fee is used for rent for the next account.',
                            'It accumulates in the account so that once the corresponding Merkle tree account is full it can be rolled over',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'rollover_threshold',
                        docs: [
                            'The threshold in percentage points when the account should be rolled over (95 corresponds to 95% filled).',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'network_fee',
                        docs: ['Tip for maintaining the account.'],
                        type: 'u64',
                    },
                    {
                        name: 'rolledover_slot',
                        docs: [
                            'The slot when the account was rolled over, a rolled over account should not be written to.',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'close_threshold',
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
            name: 'StateMerkleTreeAccount',
            docs: [
                'Concurrent state Merkle tree used for public compressed transactions.',
            ],
            serialization: 'bytemuck',
            repr: {
                kind: 'c',
            },
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'metadata',
                        type: {
                            defined: {
                                name: 'MerkleTreeMetadata',
                            },
                        },
                    },
                ],
            },
        },
    ],
};
