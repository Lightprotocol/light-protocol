export type LightRegistry = {
    version: '0.5.0';
    name: 'light_registry';
    constants: [
        {
            name: 'AUTHORITY_PDA_SEED';
            type: 'bytes';
            value: '[97, 117, 116, 104, 111, 114, 105, 116, 121]';
        },
    ];
    instructions: [
        {
            name: 'initializeGovernanceAuthority';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authorityPda';
                    isMut: true;
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
                    name: 'authority';
                    type: 'publicKey';
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
            name: 'updateGovernanceAuthority';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authorityPda';
                    isMut: true;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'newAuthority';
                    type: 'publicKey';
                },
            ];
        },
        {
            name: 'registerSystemProgram';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authorityPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'cpiAuthority';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'groupPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'programToBeRegistered';
                    isMut: false;
                    isSigner: false;
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
            name: 'nullify';
            accounts: [
                {
                    name: 'registeredForesterPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthority';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'logWrapper';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'merkleTree';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'nullifierQueue';
                    isMut: true;
                    isSigner: false;
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
            name: 'updateAddressMerkleTree';
            accounts: [
                {
                    name: 'registeredForesterPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthority';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'queue';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'merkleTree';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'logWrapper';
                    isMut: false;
                    isSigner: false;
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
            name: 'rolloverAddressMerkleTreeAndQueue';
            accounts: [
                {
                    name: 'registeredForesterPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthority';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'newMerkleTree';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'newQueue';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'oldMerkleTree';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'oldQueue';
                    isMut: true;
                    isSigner: false;
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
            accounts: [
                {
                    name: 'registeredForesterPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'cpiAuthority';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'newMerkleTree';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'newQueue';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'oldMerkleTree';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'oldQueue';
                    isMut: true;
                    isSigner: false;
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
            name: 'registerForester';
            accounts: [
                {
                    name: 'foresterEpochPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'signer';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authorityPda';
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
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'authority';
                    type: 'publicKey';
                },
            ];
        },
        {
            name: 'updateForesterEpochPda';
            accounts: [
                {
                    name: 'signer';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'foresterEpochPda';
                    isMut: true;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'authority';
                    type: 'publicKey';
                },
            ];
        },
        {
            name: 'initializeAddressMerkleTree';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                    docs: [
                        'Anyone can create new trees just the fees cannot be set arbitrarily.',
                    ];
                },
                {
                    name: 'merkleTree';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'queue';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'cpiAuthority';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'index';
                    type: 'u64';
                },
                {
                    name: 'programOwner';
                    type: {
                        option: 'publicKey';
                    };
                },
                {
                    name: 'merkleTreeConfig';
                    type: {
                        defined: 'AddressMerkleTreeConfig';
                    };
                },
                {
                    name: 'queueConfig';
                    type: {
                        defined: 'AddressQueueConfig';
                    };
                },
            ];
        },
        {
            name: 'initializeStateMerkleTree';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                    docs: [
                        'Anyone can create new trees just the fees cannot be set arbitrarily.',
                    ];
                },
                {
                    name: 'merkleTree';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'queue';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'cpiAuthority';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'index';
                    type: 'u64';
                },
                {
                    name: 'programOwner';
                    type: {
                        option: 'publicKey';
                    };
                },
                {
                    name: 'merkleTreeConfig';
                    type: {
                        defined: 'StateMerkleTreeConfig';
                    };
                },
                {
                    name: 'queueConfig';
                    type: {
                        defined: 'NullifierQueueConfig';
                    };
                },
                {
                    name: 'additionalRent';
                    type: 'u64';
                },
            ];
        },
    ];
    accounts: [
        {
            name: 'foresterEpoch';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'publicKey';
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
            name: 'lightGovernanceAuthority';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'publicKey';
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
    ];
    errors: [
        {
            code: 6000;
            name: 'InvalidForester';
            msg: 'InvalidForester';
        },
    ];
};

export const IDL: LightRegistry = {
    version: '0.5.0',
    name: 'light_registry',
    constants: [
        {
            name: 'AUTHORITY_PDA_SEED',
            type: 'bytes',
            value: '[97, 117, 116, 104, 111, 114, 105, 116, 121]',
        },
    ],
    instructions: [
        {
            name: 'initializeGovernanceAuthority',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authorityPda',
                    isMut: true,
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
                    name: 'authority',
                    type: 'publicKey',
                },
                {
                    name: 'rewards',
                    type: {
                        vec: 'u64',
                    },
                },
                {
                    name: 'bump',
                    type: 'u8',
                },
            ],
        },
        {
            name: 'updateGovernanceAuthority',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authorityPda',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'bump',
                    type: 'u8',
                },
                {
                    name: 'newAuthority',
                    type: 'publicKey',
                },
            ],
        },
        {
            name: 'registerSystemProgram',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authorityPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'cpiAuthority',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'groupPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'programToBeRegistered',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'bump',
                    type: 'u8',
                },
            ],
        },
        {
            name: 'nullify',
            accounts: [
                {
                    name: 'registeredForesterPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthority',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'logWrapper',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'merkleTree',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'nullifierQueue',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'bump',
                    type: 'u8',
                },
                {
                    name: 'changeLogIndices',
                    type: {
                        vec: 'u64',
                    },
                },
                {
                    name: 'leavesQueueIndices',
                    type: {
                        vec: 'u16',
                    },
                },
                {
                    name: 'indices',
                    type: {
                        vec: 'u64',
                    },
                },
                {
                    name: 'proofs',
                    type: {
                        vec: {
                            vec: {
                                array: ['u8', 32],
                            },
                        },
                    },
                },
            ],
        },
        {
            name: 'updateAddressMerkleTree',
            accounts: [
                {
                    name: 'registeredForesterPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthority',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'queue',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'merkleTree',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'logWrapper',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'bump',
                    type: 'u8',
                },
                {
                    name: 'changelogIndex',
                    type: 'u16',
                },
                {
                    name: 'indexedChangelogIndex',
                    type: 'u16',
                },
                {
                    name: 'value',
                    type: 'u16',
                },
                {
                    name: 'lowAddressIndex',
                    type: 'u64',
                },
                {
                    name: 'lowAddressValue',
                    type: {
                        array: ['u8', 32],
                    },
                },
                {
                    name: 'lowAddressNextIndex',
                    type: 'u64',
                },
                {
                    name: 'lowAddressNextValue',
                    type: {
                        array: ['u8', 32],
                    },
                },
                {
                    name: 'lowAddressProof',
                    type: {
                        array: [
                            {
                                array: ['u8', 32],
                            },
                            16,
                        ],
                    },
                },
            ],
        },
        {
            name: 'rolloverAddressMerkleTreeAndQueue',
            accounts: [
                {
                    name: 'registeredForesterPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthority',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'newMerkleTree',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'newQueue',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'oldMerkleTree',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'oldQueue',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'bump',
                    type: 'u8',
                },
            ],
        },
        {
            name: 'rolloverStateMerkleTreeAndQueue',
            accounts: [
                {
                    name: 'registeredForesterPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'cpiAuthority',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'newMerkleTree',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'newQueue',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'oldMerkleTree',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'oldQueue',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'bump',
                    type: 'u8',
                },
            ],
        },
        {
            name: 'registerForester',
            accounts: [
                {
                    name: 'foresterEpochPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'signer',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authorityPda',
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
                    name: 'bump',
                    type: 'u8',
                },
                {
                    name: 'authority',
                    type: 'publicKey',
                },
            ],
        },
        {
            name: 'updateForesterEpochPda',
            accounts: [
                {
                    name: 'signer',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'foresterEpochPda',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'authority',
                    type: 'publicKey',
                },
            ],
        },
        {
            name: 'initializeAddressMerkleTree',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                    docs: [
                        'Anyone can create new trees just the fees cannot be set arbitrarily.',
                    ],
                },
                {
                    name: 'merkleTree',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'queue',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'cpiAuthority',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'bump',
                    type: 'u8',
                },
                {
                    name: 'index',
                    type: 'u64',
                },
                {
                    name: 'programOwner',
                    type: {
                        option: 'publicKey',
                    },
                },
                {
                    name: 'merkleTreeConfig',
                    type: {
                        defined: 'AddressMerkleTreeConfig',
                    },
                },
                {
                    name: 'queueConfig',
                    type: {
                        defined: 'AddressQueueConfig',
                    },
                },
            ],
        },
        {
            name: 'initializeStateMerkleTree',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                    docs: [
                        'Anyone can create new trees just the fees cannot be set arbitrarily.',
                    ],
                },
                {
                    name: 'merkleTree',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'queue',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'cpiAuthority',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'bump',
                    type: 'u8',
                },
                {
                    name: 'index',
                    type: 'u64',
                },
                {
                    name: 'programOwner',
                    type: {
                        option: 'publicKey',
                    },
                },
                {
                    name: 'merkleTreeConfig',
                    type: {
                        defined: 'StateMerkleTreeConfig',
                    },
                },
                {
                    name: 'queueConfig',
                    type: {
                        defined: 'NullifierQueueConfig',
                    },
                },
                {
                    name: 'additionalRent',
                    type: 'u64',
                },
            ],
        },
    ],
    accounts: [
        {
            name: 'foresterEpoch',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'authority',
                        type: 'publicKey',
                    },
                    {
                        name: 'counter',
                        type: 'u64',
                    },
                    {
                        name: 'epochStart',
                        type: 'u64',
                    },
                    {
                        name: 'epochEnd',
                        type: 'u64',
                    },
                ],
            },
        },
        {
            name: 'lightGovernanceAuthority',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'authority',
                        type: 'publicKey',
                    },
                    {
                        name: 'bump',
                        type: 'u8',
                    },
                    {
                        name: 'epoch',
                        type: 'u64',
                    },
                    {
                        name: 'epochLength',
                        type: 'u64',
                    },
                    {
                        name: 'padding',
                        type: {
                            array: ['u8', 7],
                        },
                    },
                    {
                        name: 'rewards',
                        type: {
                            vec: 'u64',
                        },
                    },
                ],
            },
        },
    ],
    errors: [
        {
            code: 6000,
            name: 'InvalidForester',
            msg: 'InvalidForester',
        },
    ],
};
