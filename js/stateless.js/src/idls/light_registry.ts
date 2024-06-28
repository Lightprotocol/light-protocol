/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/light_registry.json`.
 */
export type LightRegistry = {
    address: '7Z9Yuy3HkBCc2Wf3xzMGnz6qpV4n7ciwcoEMGKqhAnj1';
    metadata: {
        name: 'lightRegistry';
        version: '0.5.0';
        spec: '0.1.0';
        description: 'Light core protocol logic';
        repository: 'https://github.com/Lightprotocol/light-protocol';
    };
    instructions: [
        {
            name: 'deregisterSystemProgram';
            discriminator: [11, 156, 246, 218, 141, 251, 236, 95];
            accounts: [
                {
                    name: 'authority';
                    writable: true;
                    signer: true;
                    relations: ['protocolConfigPda'];
                },
                {
                    name: 'protocolConfigPda';
                    writable: true;
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
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'registeredProgramPda';
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
            name: 'finalizeRegistration';
            docs: [
                'This transaction can be included as additional instruction in the first',
                'work instructions during the active phase.',
                'Registration Period must be over.',
            ];
            discriminator: [230, 188, 172, 96, 204, 247, 98, 227];
            accounts: [
                {
                    name: 'authority';
                    signer: true;
                    relations: ['foresterEpochPda'];
                },
                {
                    name: 'foresterEpochPda';
                    writable: true;
                },
                {
                    name: 'epochPda';
                },
            ];
            args: [];
        },
        {
            name: 'initializeAddressMerkleTree';
            discriminator: [3, 163, 248, 25, 49, 199, 115, 232];
            accounts: [
                {
                    name: 'authority';
                    docs: [
                        'Anyone can create new trees just the fees cannot be set arbitrarily.',
                    ];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'merkleTree';
                    writable: true;
                },
                {
                    name: 'queue';
                    writable: true;
                },
                {
                    name: 'registeredProgramPda';
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
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'protocolConfigPda';
                },
                {
                    name: 'cpiContextAccount';
                    optional: true;
                },
                {
                    name: 'lightSystemProgram';
                    optional: true;
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'programOwner';
                    type: {
                        option: 'pubkey';
                    };
                },
                {
                    name: 'forester';
                    type: {
                        option: 'pubkey';
                    };
                },
                {
                    name: 'merkleTreeConfig';
                    type: {
                        defined: {
                            name: 'addressMerkleTreeConfig';
                        };
                    };
                },
                {
                    name: 'queueConfig';
                    type: {
                        defined: {
                            name: 'nullifierQueueConfig';
                        };
                    };
                },
            ];
        },
        {
            name: 'initializeProtocolConfig';
            docs: [
                'Initializes the protocol config pda. Can only be called once by the',
                'program account keypair.',
            ];
            discriminator: [28, 50, 43, 233, 244, 98, 123, 118];
            accounts: [
                {
                    name: 'feePayer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    docs: [
                        'The authority should be updated to a different keypair after',
                        'initialization.',
                    ];
                    signer: true;
                },
                {
                    name: 'protocolConfigPda';
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
                {
                    name: 'selfProgram';
                    address: '7Z9Yuy3HkBCc2Wf3xzMGnz6qpV4n7ciwcoEMGKqhAnj1';
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
                        defined: {
                            name: 'protocolConfig';
                        };
                    };
                },
            ];
        },
        {
            name: 'initializeStateMerkleTree';
            discriminator: [49, 16, 53, 208, 88, 90, 196, 56];
            accounts: [
                {
                    name: 'authority';
                    docs: [
                        'Anyone can create new trees just the fees cannot be set arbitrarily.',
                    ];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'merkleTree';
                    writable: true;
                },
                {
                    name: 'queue';
                    writable: true;
                },
                {
                    name: 'registeredProgramPda';
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
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'protocolConfigPda';
                },
                {
                    name: 'cpiContextAccount';
                    optional: true;
                },
                {
                    name: 'lightSystemProgram';
                    optional: true;
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'programOwner';
                    type: {
                        option: 'pubkey';
                    };
                },
                {
                    name: 'forester';
                    type: {
                        option: 'pubkey';
                    };
                },
                {
                    name: 'merkleTreeConfig';
                    type: {
                        defined: {
                            name: 'stateMerkleTreeConfig';
                        };
                    };
                },
                {
                    name: 'queueConfig';
                    type: {
                        defined: {
                            name: 'nullifierQueueConfig';
                        };
                    };
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
                    optional: true;
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
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
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
                    name: 'feePayer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                    relations: ['protocolConfigPda'];
                },
                {
                    name: 'protocolConfigPda';
                },
                {
                    name: 'foresterPda';
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [102, 111, 114, 101, 115, 116, 101, 114];
                            },
                            {
                                kind: 'arg';
                                path: 'foresterAuthority';
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
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'authority';
                    type: 'pubkey';
                },
                {
                    name: 'config';
                    type: {
                        defined: {
                            name: 'foresterConfig';
                        };
                    };
                },
                {
                    name: 'weight';
                    type: {
                        option: 'u64';
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
            discriminator: [43, 120, 253, 194, 109, 192, 101, 188];
            accounts: [
                {
                    name: 'feePayer';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                    relations: ['foresterPda'];
                },
                {
                    name: 'foresterPda';
                },
                {
                    name: 'foresterEpochPda';
                    docs: [
                        'Instruction checks that current_epoch is the the current epoch and that',
                        'the epoch is in registration phase.',
                    ];
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
                                kind: 'account';
                                path: 'foresterPda';
                            },
                            {
                                kind: 'arg';
                                path: 'currentEpoch';
                            },
                        ];
                    };
                },
                {
                    name: 'protocolConfig';
                },
                {
                    name: 'epochPda';
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'arg';
                                path: 'currentEpoch';
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
                    name: 'epoch';
                    type: 'u64';
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
                    relations: ['protocolConfigPda'];
                },
                {
                    name: 'protocolConfigPda';
                    writable: true;
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
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
                {
                    name: 'registeredProgramPda';
                    writable: true;
                },
                {
                    name: 'programToBeRegistered';
                    docs: [
                        '- is signer so that only the program deployer can register a program.',
                    ];
                    signer: true;
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
            name: 'reportWork';
            discriminator: [170, 110, 232, 47, 145, 213, 138, 162];
            accounts: [
                {
                    name: 'authority';
                    signer: true;
                    relations: ['foresterEpochPda'];
                },
                {
                    name: 'foresterEpochPda';
                    writable: true;
                },
                {
                    name: 'epochPda';
                    writable: true;
                },
            ];
            args: [];
        },
        {
            name: 'rolloverAddressMerkleTreeAndQueue';
            discriminator: [24, 84, 27, 12, 134, 166, 23, 192];
            accounts: [
                {
                    name: 'registeredForesterPda';
                    writable: true;
                    optional: true;
                },
                {
                    name: 'authority';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'cpiAuthority';
                },
                {
                    name: 'registeredProgramPda';
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
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
                    optional: true;
                },
                {
                    name: 'authority';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'cpiAuthority';
                },
                {
                    name: 'registeredProgramPda';
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
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
                {
                    name: 'cpiContextAccount';
                },
                {
                    name: 'lightSystemProgram';
                    address: 'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN';
                },
                {
                    name: 'protocolConfigPda';
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
                    optional: true;
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
                },
                {
                    name: 'accountCompressionProgram';
                    address: 'CbjvJc1SNx1aav8tU49dJGHu8EUdzQJSMtkjDmV8miqK';
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
            name: 'updateForesterPda';
            discriminator: [246, 179, 30, 239, 171, 39, 57, 171];
            accounts: [
                {
                    name: 'authority';
                    signer: true;
                    relations: ['foresterPda'];
                },
                {
                    name: 'foresterPda';
                    writable: true;
                },
                {
                    name: 'newAuthority';
                    signer: true;
                    optional: true;
                },
            ];
            args: [
                {
                    name: 'config';
                    type: {
                        option: {
                            defined: {
                                name: 'foresterConfig';
                            };
                        };
                    };
                },
            ];
        },
        {
            name: 'updateForesterPdaWeight';
            discriminator: [227, 190, 126, 130, 203, 247, 54, 43];
            accounts: [
                {
                    name: 'authority';
                    signer: true;
                    relations: ['protocolConfigPda'];
                },
                {
                    name: 'protocolConfigPda';
                },
                {
                    name: 'foresterPda';
                    writable: true;
                },
            ];
            args: [
                {
                    name: 'newWeight';
                    type: 'u64';
                },
            ];
        },
        {
            name: 'updateProtocolConfig';
            discriminator: [197, 97, 123, 54, 221, 168, 11, 135];
            accounts: [
                {
                    name: 'feePayer';
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                    relations: ['protocolConfigPda'];
                },
                {
                    name: 'protocolConfigPda';
                    writable: true;
                },
                {
                    name: 'newAuthority';
                    signer: true;
                    optional: true;
                },
            ];
            args: [
                {
                    name: 'protocolConfig';
                    type: {
                        option: {
                            defined: {
                                name: 'protocolConfig';
                            };
                        };
                    };
                },
            ];
        },
    ];
    accounts: [
        {
            name: 'addressMerkleTreeAccount';
            discriminator: [11, 161, 175, 9, 212, 229, 73, 73];
        },
        {
            name: 'epochPda';
            discriminator: [66, 224, 46, 2, 167, 137, 120, 107];
        },
        {
            name: 'foresterEpochPda';
            discriminator: [29, 117, 211, 141, 99, 143, 250, 114];
        },
        {
            name: 'foresterPda';
            discriminator: [51, 47, 187, 86, 82, 153, 117, 5];
        },
        {
            name: 'groupAuthority';
            discriminator: [15, 207, 4, 160, 127, 38, 142, 162];
        },
        {
            name: 'protocolConfigPda';
            discriminator: [96, 176, 239, 146, 1, 254, 99, 146];
        },
        {
            name: 'stateMerkleTreeAccount';
            discriminator: [172, 43, 172, 186, 29, 73, 219, 84];
        },
    ];
    errors: [
        {
            code: 6000;
            name: 'invalidForester';
            msg: 'invalidForester';
        },
        {
            code: 6001;
            name: 'notInReportWorkPhase';
        },
        {
            code: 6002;
            name: 'stakeAccountAlreadySynced';
        },
        {
            code: 6003;
            name: 'epochEnded';
        },
        {
            code: 6004;
            name: 'foresterNotEligible';
        },
        {
            code: 6005;
            name: 'notInRegistrationPeriod';
        },
        {
            code: 6006;
            name: 'weightInsuffient';
        },
        {
            code: 6007;
            name: 'foresterAlreadyRegistered';
        },
        {
            code: 6008;
            name: 'invalidEpochAccount';
        },
        {
            code: 6009;
            name: 'invalidEpoch';
        },
        {
            code: 6010;
            name: 'epochStillInProgress';
        },
        {
            code: 6011;
            name: 'notInActivePhase';
        },
        {
            code: 6012;
            name: 'foresterAlreadyReportedWork';
        },
        {
            code: 6013;
            name: 'invalidNetworkFee';
        },
        {
            code: 6014;
            name: 'finalizeCounterExceeded';
        },
        {
            code: 6015;
            name: 'cpiContextAccountMissing';
        },
        {
            code: 6016;
            name: 'arithmeticUnderflow';
        },
        {
            code: 6017;
            name: 'registrationNotFinalized';
        },
        {
            code: 6018;
            name: 'cpiContextAccountInvalidDataLen';
        },
        {
            code: 6019;
            name: 'invalidConfigUpdate';
        },
        {
            code: 6020;
            name: 'invalidSigner';
        },
        {
            code: 6021;
            name: 'getLatestedRegisterEpochFailed';
        },
        {
            code: 6022;
            name: 'getLatestActiveEpochFailed';
        },
        {
            code: 6023;
            name: 'foresterUndefined';
        },
        {
            code: 6024;
            name: 'foresterDefined';
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
            name: 'addressMerkleTreeAccount';
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
        {
            name: 'addressMerkleTreeConfig';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'height';
                        type: 'u32';
                    },
                    {
                        name: 'changelogSize';
                        type: 'u64';
                    },
                    {
                        name: 'rootsSize';
                        type: 'u64';
                    },
                    {
                        name: 'canopyDepth';
                        type: 'u64';
                    },
                    {
                        name: 'addressChangelogSize';
                        type: 'u64';
                    },
                    {
                        name: 'networkFee';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'rolloverThreshold';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'closeThreshold';
                        type: {
                            option: 'u64';
                        };
                    },
                ];
            };
        },
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
                            defined: {
                                name: 'protocolConfig';
                            };
                        };
                    },
                    {
                        name: 'totalWork';
                        type: 'u64';
                    },
                    {
                        name: 'registeredWeight';
                        type: 'u64';
                    },
                ];
            };
        },
        {
            name: 'foresterConfig';
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
            name: 'foresterEpochPda';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'pubkey';
                    },
                    {
                        name: 'config';
                        type: {
                            defined: {
                                name: 'foresterConfig';
                            };
                        };
                    },
                    {
                        name: 'epoch';
                        type: 'u64';
                    },
                    {
                        name: 'weight';
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
                            'rewards.',
                        ];
                        type: 'bool';
                    },
                    {
                        name: 'foresterIndex';
                        docs: [
                            'Start index of the range that determines when the forester is eligible to perform work.',
                            'End index is forester_start_index + weight',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'epochActivePhaseStartSlot';
                        type: 'u64';
                    },
                    {
                        name: 'totalEpochWeight';
                        docs: [
                            'Total epoch weight is registered weight of the epoch account after',
                            'registration is concluded and active epoch period starts.',
                        ];
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'protocolConfig';
                        type: {
                            defined: {
                                name: 'protocolConfig';
                            };
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
            name: 'foresterPda';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'pubkey';
                    },
                    {
                        name: 'config';
                        type: {
                            defined: {
                                name: 'foresterConfig';
                            };
                        };
                    },
                    {
                        name: 'activeWeight';
                        type: 'u64';
                    },
                    {
                        name: 'pendingWeight';
                        docs: [
                            'Pending weight which will get active once the next epoch starts.',
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
            name: 'nullifierQueueConfig';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'capacity';
                        type: 'u16';
                    },
                    {
                        name: 'sequenceThreshold';
                        type: 'u64';
                    },
                    {
                        name: 'networkFee';
                        type: {
                            option: 'u64';
                        };
                    },
                ];
            };
        },
        {
            name: 'protocolConfig';
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
                        name: 'minWeight';
                        docs: [
                            'Minimum weight required for a forester to register to an epoch.',
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'slotLength';
                        docs: ['Light protocol slot length.'];
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
                        ];
                        type: 'u64';
                    },
                    {
                        name: 'networkFee';
                        type: 'u64';
                    },
                    {
                        name: 'cpiContextSize';
                        type: 'u64';
                    },
                    {
                        name: 'finalizeCounterLimit';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolder';
                        docs: ['Placeholder for future protocol updates.'];
                        type: 'pubkey';
                    },
                    {
                        name: 'placeHolderA';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolderB';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolderC';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolderD';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolderE';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolderF';
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
                        type: 'pubkey';
                    },
                    {
                        name: 'bump';
                        type: 'u8';
                    },
                    {
                        name: 'config';
                        type: {
                            defined: {
                                name: 'protocolConfig';
                            };
                        };
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
        {
            name: 'stateMerkleTreeConfig';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'height';
                        type: 'u32';
                    },
                    {
                        name: 'changelogSize';
                        type: 'u64';
                    },
                    {
                        name: 'rootsSize';
                        type: 'u64';
                    },
                    {
                        name: 'canopyDepth';
                        type: 'u64';
                    },
                    {
                        name: 'networkFee';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'rolloverThreshold';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'closeThreshold';
                        type: {
                            option: 'u64';
                        };
                    },
                ];
            };
        },
    ];
    constants: [
        {
            name: 'foresterEpochSeed';
            type: 'bytes';
            value: '[102, 111, 114, 101, 115, 116, 101, 114, 95, 101, 112, 111, 99, 104]';
        },
        {
            name: 'foresterSeed';
            type: 'bytes';
            value: '[102, 111, 114, 101, 115, 116, 101, 114]';
        },
        {
            name: 'protocolConfigPdaSeed';
            type: 'bytes';
            value: '[97, 117, 116, 104, 111, 114, 105, 116, 121]';
        },
    ];
};
