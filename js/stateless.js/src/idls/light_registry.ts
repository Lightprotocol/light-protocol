/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/light_registry.json`.
 */
export type LightRegistry = {
    address: 'HEgU8UpKrVfXGkZRp7hYpW9Amp6p4NgJ3dM9SaH2Hso1';
    metadata: {
        name: 'lightRegistry';
        version: '0.4.1';
        spec: '0.1.0';
        description: 'Light core protocol logic';
        repository: 'https://github.com/Lightprotocol/light-protocol';
    };
    instructions: [
        {
            name: 'initializeGovernanceAuthority';
            discriminator: [72, 76, 248, 10, 175, 86, 82, 37];
            accounts: [
                {
                    name: 'authority';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authorityPda';
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
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
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
            ];
            args: [
                {
                    name: 'authority';
                    type: 'pubkey';
                },
                {
                    name: 'rewards';
                    type: {
                        vec: 'u64';
                    };
                },
                {
                    name: 'bump';
                    type: 'u8';
                },
            ];
        },
        {
            name: 'nullify';
            discriminator: [207, 160, 198, 75, 227, 146, 128, 1];
            accounts: [
                {
                    name: 'registeredForesterPda';
                    writable: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpiAuthority';
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
                    name: 'registeredProgramPda';
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    241,
                                    60,
                                    42,
                                    235,
                                    133,
                                    33,
                                    17,
                                    165,
                                    183,
                                    234,
                                    69,
                                    88,
                                    95,
                                    110,
                                    23,
                                    251,
                                    143,
                                    3,
                                    180,
                                    58,
                                    254,
                                    196,
                                    196,
                                    124,
                                    177,
                                    220,
                                    184,
                                    225,
                                    215,
                                    132,
                                    249,
                                    196,
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
                    name: 'accountCompressionProgram';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'logWrapper';
                },
                {
                    name: 'merkleTree';
                    writable: true;
                },
                {
                    name: 'nullifierQueue';
                    writable: true;
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'changeLogIndices';
                    type: {
                        vec: 'u64';
                    };
                },
                {
                    name: 'leavesQueueIndices';
                    type: {
                        vec: 'u16';
                    };
                },
                {
                    name: 'indices';
                    type: {
                        vec: 'u64';
                    };
                },
                {
                    name: 'proofs';
                    type: {
                        vec: {
                            vec: {
                                array: ['u8', 32];
                            };
                        };
                    };
                },
            ];
        },
        {
            name: 'registerForester';
            discriminator: [62, 47, 240, 103, 84, 200, 226, 73];
            accounts: [
                {
                    name: 'foresterEpochPda';
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    102,
                                    111,
                                    114,
                                    101,
                                    115,
                                    116,
                                    101,
                                    114,
                                    95,
                                    101,
                                    112,
                                    111,
                                    99,
                                    104,
                                ];
                            },
                            {
                                kind: 'arg';
                                path: 'authority';
                            },
                        ];
                    };
                },
                {
                    name: 'signer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authorityPda';
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'authority';
                    type: 'pubkey';
                },
            ];
        },
        {
            name: 'registerSystemProgram';
            discriminator: [10, 100, 93, 53, 172, 229, 7, 244];
            accounts: [
                {
                    name: 'authority';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authorityPda';
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
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
                    name: 'cpiAuthority';
                    writable: true;
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
                    name: 'groupPda';
                    writable: true;
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
                {
                    name: 'registeredProgramPda';
                },
                {
                    name: 'programToBeRegistered';
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
            ];
        },
        {
            name: 'rolloverAddressMerkleTreeAndQueue';
            discriminator: [24, 84, 27, 12, 134, 166, 23, 192];
            accounts: [
                {
                    name: 'registeredForesterPda';
                    writable: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpiAuthority';
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
                    name: 'registeredProgramPda';
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    241,
                                    60,
                                    42,
                                    235,
                                    133,
                                    33,
                                    17,
                                    165,
                                    183,
                                    234,
                                    69,
                                    88,
                                    95,
                                    110,
                                    23,
                                    251,
                                    143,
                                    3,
                                    180,
                                    58,
                                    254,
                                    196,
                                    196,
                                    124,
                                    177,
                                    220,
                                    184,
                                    225,
                                    215,
                                    132,
                                    249,
                                    196,
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
                    name: 'accountCompressionProgram';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'newMerkleTree';
                    writable: true;
                },
                {
                    name: 'newQueue';
                    writable: true;
                },
                {
                    name: 'oldMerkleTree';
                    writable: true;
                },
                {
                    name: 'oldQueue';
                    writable: true;
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
            ];
        },
        {
            name: 'rolloverStateMerkleTreeAndQueue';
            discriminator: [110, 28, 22, 15, 48, 90, 127, 210];
            accounts: [
                {
                    name: 'registeredForesterPda';
                    writable: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpiAuthority';
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
                    name: 'registeredProgramPda';
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    241,
                                    60,
                                    42,
                                    235,
                                    133,
                                    33,
                                    17,
                                    165,
                                    183,
                                    234,
                                    69,
                                    88,
                                    95,
                                    110,
                                    23,
                                    251,
                                    143,
                                    3,
                                    180,
                                    58,
                                    254,
                                    196,
                                    196,
                                    124,
                                    177,
                                    220,
                                    184,
                                    225,
                                    215,
                                    132,
                                    249,
                                    196,
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
                    name: 'accountCompressionProgram';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'newMerkleTree';
                    writable: true;
                },
                {
                    name: 'newQueue';
                    writable: true;
                },
                {
                    name: 'oldMerkleTree';
                    writable: true;
                },
                {
                    name: 'oldQueue';
                    writable: true;
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
            ];
        },
        {
            name: 'updateAddressMerkleTree';
            discriminator: [75, 208, 63, 56, 207, 74, 124, 18];
            accounts: [
                {
                    name: 'registeredForesterPda';
                    writable: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'cpiAuthority';
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
                    name: 'registeredProgramPda';
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    241,
                                    60,
                                    42,
                                    235,
                                    133,
                                    33,
                                    17,
                                    165,
                                    183,
                                    234,
                                    69,
                                    88,
                                    95,
                                    110,
                                    23,
                                    251,
                                    143,
                                    3,
                                    180,
                                    58,
                                    254,
                                    196,
                                    196,
                                    124,
                                    177,
                                    220,
                                    184,
                                    225,
                                    215,
                                    132,
                                    249,
                                    196,
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
                    name: 'accountCompressionProgram';
                    address: 'M9w4GyHwyaZJUhsTC5vu6Hcvm4kUe63VKXdRWmMbJ3U';
                },
                {
                    name: 'queue';
                    writable: true;
                },
                {
                    name: 'merkleTree';
                    writable: true;
                },
                {
                    name: 'logWrapper';
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'changelogIndex';
                    type: 'u16';
                },
                {
                    name: 'indexedChangelogIndex';
                    type: 'u16';
                },
                {
                    name: 'value';
                    type: 'u16';
                },
                {
                    name: 'lowAddressIndex';
                    type: 'u64';
                },
                {
                    name: 'lowAddressValue';
                    type: {
                        array: ['u8', 32];
                    };
                },
                {
                    name: 'lowAddressNextIndex';
                    type: 'u64';
                },
                {
                    name: 'lowAddressNextValue';
                    type: {
                        array: ['u8', 32];
                    };
                },
                {
                    name: 'lowAddressProof';
                    type: {
                        array: [
                            {
                                array: ['u8', 32];
                            },
                            16,
                        ];
                    };
                },
            ];
        },
        {
            name: 'updateForesterEpochPda';
            discriminator: [191, 203, 90, 97, 203, 203, 227, 225];
            accounts: [
                {
                    name: 'signer';
                    signer: true;
                },
                {
                    name: 'foresterEpochPda';
                    writable: true;
                },
            ];
            args: [
                {
                    name: 'authority';
                    type: 'pubkey';
                },
            ];
        },
        {
            name: 'updateGovernanceAuthority';
            discriminator: [11, 185, 227, 55, 39, 32, 168, 14];
            accounts: [
                {
                    name: 'authority';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authorityPda';
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
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
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'newAuthority';
                    type: 'pubkey';
                },
            ];
        },
    ];
    accounts: [
        {
            name: 'foresterEpoch';
            discriminator: [113, 133, 8, 112, 180, 37, 115, 207];
        },
        {
            name: 'groupAuthority';
            discriminator: [15, 207, 4, 160, 127, 38, 142, 162];
        },
        {
            name: 'lightGovernanceAuthority';
            discriminator: [247, 101, 118, 106, 123, 10, 47, 145];
        },
        {
            name: 'registeredProgram';
            discriminator: [31, 251, 180, 235, 3, 116, 50, 4];
        },
    ];
    errors: [
        {
            code: 6000;
            name: 'invalidForester';
            msg: 'invalidForester';
        },
    ];
    types: [
        {
            name: 'foresterEpoch';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'pubkey';
                    },
                    {
                        name: 'counter';
                        type: 'u64';
                    },
                    {
                        name: 'epochStart';
                        type: 'u64';
                    },
                    {
                        name: 'epochEnd';
                        type: 'u64';
                    },
                ];
            };
        },
        {
            name: 'groupAuthority';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'pubkey';
                    },
                    {
                        name: 'seed';
                        type: 'pubkey';
                    },
                ];
            };
        },
        {
            name: 'lightGovernanceAuthority';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'pubkey';
                    },
                    {
                        name: 'bump';
                        type: 'u8';
                    },
                    {
                        name: 'epoch';
                        type: 'u64';
                    },
                    {
                        name: 'epochLength';
                        type: 'u64';
                    },
                    {
                        name: 'padding';
                        type: {
                            array: ['u8', 7];
                        };
                    },
                    {
                        name: 'rewards';
                        type: {
                            vec: 'u64';
                        };
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
    ];
    constants: [
        {
            name: 'authorityPdaSeed';
            type: 'bytes';
            value: '[97, 117, 116, 104, 111, 114, 105, 116, 121]';
        },
    ];
};
