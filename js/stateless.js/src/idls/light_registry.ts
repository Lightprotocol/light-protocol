export type LightRegistry = {
    version: '0.4.1';
    name: 'light_registry';
    constants: [
        {
            name: 'PROTOCOL_CONFIG_PDA_SEED';
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
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'protocolConfig';
                    type: {
                        defined: 'ProtocolConfig';
                    };
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
                {
                    name: 'newAuthority';
                    isMut: false;
                    isSigner: true;
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
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'programToBeRegistered';
                    isMut: false;
                    isSigner: true;
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
                    name: 'foresterPda';
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
                {
                    name: 'config';
                    type: {
                        defined: 'ForesterConfig';
                    };
                },
            ];
        },
        {
            name: 'updateForester';
            accounts: [
                {
                    name: 'foresterPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'newAuthority';
                    isMut: false;
                    isSigner: true;
                    isOptional: true;
                },
            ];
            args: [
                {
                    name: 'config';
                    type: {
                        defined: 'ForesterConfig';
                    };
                },
            ];
        },
        {
            name: 'registerForesterEpoch';
            docs: [
                'Registers the forester for the epoch.',
                '1. Only the forester can register herself for the epoch.',
                '2. Protocol config is copied.',
                '3. Epoch account is created if needed.',
            ];
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'foresterPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'foresterEpochPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'protocolConfig';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'epochPda';
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
                    name: 'epoch';
                    type: 'u64';
                },
            ];
        },
        {
            name: 'finalizeRegistration';
            docs: [
                'This transaction can be included as additional instruction in the first',
                'work instructions during the active phase.',
                'Registration Period must be over.',
                'TODO: introduce grace period between registration and before',
                "active phase starts, do I really need it or isn't it clear who gets the",
                'first slot the first sign up?',
            ];
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'foresterEpochPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'epochPda';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [];
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
            name: 'reportWork';
            accounts: [
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'foresterEpochPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'epochPda';
                    isMut: true;
                    isSigner: false;
                },
            ];
            args: [];
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
            name: 'epochPda';
            docs: ['Is used for tallying and rewards calculation'];
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'epoch';
                        type: 'u64';
                    },
                    {
                        name: 'protocolConfig';
                        type: {
                            defined: 'ProtocolConfig';
                        };
                    },
                    {
                        name: 'totalWork';
                        type: 'u64';
                    },
                    {
                        name: 'registeredStake';
                        type: 'u64';
                    },
                ];
            };
        },
        {
            name: 'foresterEpochPda';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'publicKey';
                    },
                    {
                        name: 'config';
                        type: {
                            defined: 'ForesterConfig';
                        };
                    },
                    {
                        name: 'epoch';
                        type: 'u64';
                    },
                    {
                        name: 'stakeWeight';
                        type: 'u64';
                    },
                    {
                        name: 'workCounter';
                        type: 'u64';
                    },
                    {
                        name: 'hasReportedWork';
                        docs: [
                            'Work can be reported in an extra round to earn extra performance based',
                            'rewards. // TODO: make sure that performance based rewards can only be',
                            'claimed if work has been reported',
                        ];
                        type: 'bool';
                    },
                    {
                        name: 'foresterIndex';
                        docs: [
                            'Start index of the range that determines when the forester is eligible to perform work.',
                            'End index is forester_start_index + stake_weight',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'epochActivePhaseStartSlot';
                        type: 'u64';
                    },
                    {
                        name: 'totalEpochStateWeight';
                        docs: [
                            'Total epoch state weight is registered stake of the epoch account after',
                            'registration is concluded and active epoch period starts.',
                        ];
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'protocolConfig';
                        type: {
                            defined: 'ProtocolConfig';
                        };
                    },
                    {
                        name: 'finalizeCounter';
                        docs: [
                            'Incremented every time finalize registration is called.',
                        ];
                        type: 'u64';
                    },
                ];
            };
        },
        {
            name: 'protocolConfigPda';
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
                        name: 'config';
                        type: {
                            defined: 'ProtocolConfig';
                        };
                    },
                ];
            };
        },
        {
            name: 'foresterAccount';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'publicKey';
                    },
                    {
                        name: 'config';
                        type: {
                            defined: 'ForesterConfig';
                        };
                    },
                    {
                        name: 'activeStakeWeight';
                        type: 'u64';
                    },
                    {
                        name: 'pendingStakeWeight';
                        docs: [
                            'Pending stake which will get active once the next epoch starts.',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'currentEpoch';
                        type: 'u64';
                    },
                    {
                        name: 'lastCompressedForesterEpochPdaHash';
                        docs: [
                            'Link to previous compressed forester epoch account hash.',
                        ];
                        type: {
                            array: ['u8', 32];
                        };
                    },
                    {
                        name: 'lastRegisteredEpoch';
                        type: 'u64';
                    },
                ];
            };
        },
    ];
    types: [
        {
            name: 'ProtocolConfig';
            docs: [
                'Epoch Phases:',
                '1. Registration',
                '2. Active',
                '3. Report Work',
                '4. Post (Epoch has ended, and rewards can be claimed.)',
                '- There is always an active phase in progress, registration and report work',
                'phases run in parallel to a currently active phase.',
            ];
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'genesisSlot';
                        docs: [
                            'Solana slot when the protocol starts operating.',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'epochReward';
                        docs: ['Total rewards per epoch.'];
                        type: 'u64';
                    },
                    {
                        name: 'baseReward';
                        docs: [
                            'Base reward for foresters, the difference between epoch reward and base',
                            'reward distributed based on performance.',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'minStake';
                        docs: [
                            'Minimum stake required for a forester to register to an epoch.',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'slotLength';
                        docs: [
                            'Light protocol slot length. (Naming is confusing for Solana slot.)',
                            'TODO: rename to epoch_length (registration + active phase length)',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'registrationPhaseLength';
                        docs: ['Foresters can register for this phase.'];
                        type: 'u64';
                    },
                    {
                        name: 'activePhaseLength';
                        docs: ['Foresters can perform work in this phase.'];
                        type: 'u64';
                    },
                    {
                        name: 'reportWorkPhaseLength';
                        docs: [
                            'Foresters can report work to receive performance based rewards in this',
                            'phase.',
                            'TODO: enforce report work == registration phase length so that',
                            'epoch in report work phase is registration epoch - 1',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'mint';
                        type: 'publicKey';
                    },
                ];
            };
        },
        {
            name: 'ForesterConfig';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'fee';
                        docs: ['Fee in percentage points.'];
                        type: 'u64';
                    },
                ];
            };
        },
        {
            name: 'EpochState';
            type: {
                kind: 'enum';
                variants: [
                    {
                        name: 'Registration';
                    },
                    {
                        name: 'Active';
                    },
                    {
                        name: 'ReportWork';
                    },
                    {
                        name: 'Post';
                    },
                    {
                        name: 'Pre';
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
        {
            code: 6001;
            name: 'NotInReportWorkPhase';
        },
        {
            code: 6002;
            name: 'StakeAccountAlreadySynced';
        },
        {
            code: 6003;
            name: 'EpochEnded';
        },
        {
            code: 6004;
            name: 'ForresterNotEligible';
        },
        {
            code: 6005;
            name: 'NotInRegistrationPeriod';
        },
        {
            code: 6006;
            name: 'StakeInsuffient';
        },
        {
            code: 6007;
            name: 'ForesterAlreadyRegistered';
        },
        {
            code: 6008;
            name: 'InvalidEpochAccount';
        },
        {
            code: 6009;
            name: 'InvalidEpoch';
        },
        {
            code: 6010;
            name: 'EpochStillInProgress';
        },
        {
            code: 6011;
            name: 'NotInActivePhase';
        },
        {
            code: 6012;
            name: 'ForesterAlreadyReportedWork';
        },
    ];
};

export const IDL: LightRegistry = {
    version: '0.4.1',
    name: 'light_registry',
    constants: [
        {
            name: 'PROTOCOL_CONFIG_PDA_SEED',
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
                    name: 'bump',
                    type: 'u8',
                },
                {
                    name: 'protocolConfig',
                    type: {
                        defined: 'ProtocolConfig',
                    },
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
                {
                    name: 'newAuthority',
                    isMut: false,
                    isSigner: true,
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
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'programToBeRegistered',
                    isMut: false,
                    isSigner: true,
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
                    name: 'foresterPda',
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
                {
                    name: 'config',
                    type: {
                        defined: 'ForesterConfig',
                    },
                },
            ],
        },
        {
            name: 'updateForester',
            accounts: [
                {
                    name: 'foresterPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'newAuthority',
                    isMut: false,
                    isSigner: true,
                    isOptional: true,
                },
            ],
            args: [
                {
                    name: 'config',
                    type: {
                        defined: 'ForesterConfig',
                    },
                },
            ],
        },
        {
            name: 'registerForesterEpoch',
            docs: [
                'Registers the forester for the epoch.',
                '1. Only the forester can register herself for the epoch.',
                '2. Protocol config is copied.',
                '3. Epoch account is created if needed.',
            ],
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'foresterPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'foresterEpochPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'protocolConfig',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'epochPda',
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
                    name: 'epoch',
                    type: 'u64',
                },
            ],
        },
        {
            name: 'finalizeRegistration',
            docs: [
                'This transaction can be included as additional instruction in the first',
                'work instructions during the active phase.',
                'Registration Period must be over.',
                'TODO: introduce grace period between registration and before',
                "active phase starts, do I really need it or isn't it clear who gets the",
                'first slot the first sign up?',
            ],
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'foresterEpochPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'epochPda',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [],
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
            name: 'reportWork',
            accounts: [
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'foresterEpochPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'epochPda',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [],
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
            name: 'epochPda',
            docs: ['Is used for tallying and rewards calculation'],
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'epoch',
                        type: 'u64',
                    },
                    {
                        name: 'protocolConfig',
                        type: {
                            defined: 'ProtocolConfig',
                        },
                    },
                    {
                        name: 'totalWork',
                        type: 'u64',
                    },
                    {
                        name: 'registeredStake',
                        type: 'u64',
                    },
                ],
            },
        },
        {
            name: 'foresterEpochPda',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'authority',
                        type: 'publicKey',
                    },
                    {
                        name: 'config',
                        type: {
                            defined: 'ForesterConfig',
                        },
                    },
                    {
                        name: 'epoch',
                        type: 'u64',
                    },
                    {
                        name: 'stakeWeight',
                        type: 'u64',
                    },
                    {
                        name: 'workCounter',
                        type: 'u64',
                    },
                    {
                        name: 'hasReportedWork',
                        docs: [
                            'Work can be reported in an extra round to earn extra performance based',
                            'rewards. // TODO: make sure that performance based rewards can only be',
                            'claimed if work has been reported',
                        ],
                        type: 'bool',
                    },
                    {
                        name: 'foresterIndex',
                        docs: [
                            'Start index of the range that determines when the forester is eligible to perform work.',
                            'End index is forester_start_index + stake_weight',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'epochActivePhaseStartSlot',
                        type: 'u64',
                    },
                    {
                        name: 'totalEpochStateWeight',
                        docs: [
                            'Total epoch state weight is registered stake of the epoch account after',
                            'registration is concluded and active epoch period starts.',
                        ],
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'protocolConfig',
                        type: {
                            defined: 'ProtocolConfig',
                        },
                    },
                    {
                        name: 'finalizeCounter',
                        docs: [
                            'Incremented every time finalize registration is called.',
                        ],
                        type: 'u64',
                    },
                ],
            },
        },
        {
            name: 'protocolConfigPda',
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
                        name: 'config',
                        type: {
                            defined: 'ProtocolConfig',
                        },
                    },
                ],
            },
        },
        {
            name: 'foresterAccount',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'authority',
                        type: 'publicKey',
                    },
                    {
                        name: 'config',
                        type: {
                            defined: 'ForesterConfig',
                        },
                    },
                    {
                        name: 'activeStakeWeight',
                        type: 'u64',
                    },
                    {
                        name: 'pendingStakeWeight',
                        docs: [
                            'Pending stake which will get active once the next epoch starts.',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'currentEpoch',
                        type: 'u64',
                    },
                    {
                        name: 'lastCompressedForesterEpochPdaHash',
                        docs: [
                            'Link to previous compressed forester epoch account hash.',
                        ],
                        type: {
                            array: ['u8', 32],
                        },
                    },
                    {
                        name: 'lastRegisteredEpoch',
                        type: 'u64',
                    },
                ],
            },
        },
    ],
    types: [
        {
            name: 'ProtocolConfig',
            docs: [
                'Epoch Phases:',
                '1. Registration',
                '2. Active',
                '3. Report Work',
                '4. Post (Epoch has ended, and rewards can be claimed.)',
                '- There is always an active phase in progress, registration and report work',
                'phases run in parallel to a currently active phase.',
            ],
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'genesisSlot',
                        docs: [
                            'Solana slot when the protocol starts operating.',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'epochReward',
                        docs: ['Total rewards per epoch.'],
                        type: 'u64',
                    },
                    {
                        name: 'baseReward',
                        docs: [
                            'Base reward for foresters, the difference between epoch reward and base',
                            'reward distributed based on performance.',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'minStake',
                        docs: [
                            'Minimum stake required for a forester to register to an epoch.',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'slotLength',
                        docs: [
                            'Light protocol slot length. (Naming is confusing for Solana slot.)',
                            'TODO: rename to epoch_length (registration + active phase length)',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'registrationPhaseLength',
                        docs: ['Foresters can register for this phase.'],
                        type: 'u64',
                    },
                    {
                        name: 'activePhaseLength',
                        docs: ['Foresters can perform work in this phase.'],
                        type: 'u64',
                    },
                    {
                        name: 'reportWorkPhaseLength',
                        docs: [
                            'Foresters can report work to receive performance based rewards in this',
                            'phase.',
                            'TODO: enforce report work == registration phase length so that',
                            'epoch in report work phase is registration epoch - 1',
                        ],
                        type: 'u64',
                    },
                    {
                        name: 'mint',
                        type: 'publicKey',
                    },
                ],
            },
        },
        {
            name: 'ForesterConfig',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'fee',
                        docs: ['Fee in percentage points.'],
                        type: 'u64',
                    },
                ],
            },
        },
        {
            name: 'EpochState',
            type: {
                kind: 'enum',
                variants: [
                    {
                        name: 'Registration',
                    },
                    {
                        name: 'Active',
                    },
                    {
                        name: 'ReportWork',
                    },
                    {
                        name: 'Post',
                    },
                    {
                        name: 'Pre',
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
        {
            code: 6001,
            name: 'NotInReportWorkPhase',
        },
        {
            code: 6002,
            name: 'StakeAccountAlreadySynced',
        },
        {
            code: 6003,
            name: 'EpochEnded',
        },
        {
            code: 6004,
            name: 'ForresterNotEligible',
        },
        {
            code: 6005,
            name: 'NotInRegistrationPeriod',
        },
        {
            code: 6006,
            name: 'StakeInsuffient',
        },
        {
            code: 6007,
            name: 'ForesterAlreadyRegistered',
        },
        {
            code: 6008,
            name: 'InvalidEpochAccount',
        },
        {
            code: 6009,
            name: 'InvalidEpoch',
        },
        {
            code: 6010,
            name: 'EpochStillInProgress',
        },
        {
            code: 6011,
            name: 'NotInActivePhase',
        },
        {
            code: 6012,
            name: 'ForesterAlreadyReportedWork',
        },
    ],
};
