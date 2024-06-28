/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/account_compression.json`.
 */
export type AccountCompression = {
    address: 'EJb9Svap6x9P2psyLW6YrDuygmMpSsiNbmZw72eDCxd7';
    metadata: {
        name: 'accountCompression';
        version: '0.4.1';
        spec: '0.1.0';
        description: 'Solana account compression program';
        repository: 'https://github.com/Lightprotocol/light-protocol';
    };
    instructions: [
        {
            name: 'initializeAddressMerkleTreeAndQueue';
            discriminator: [19, 14, 102, 183, 214, 35, 211, 13];
            accounts: [
                {
                    name: 'authority';
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
            ];
            args: [
                {
                    name: 'index';
                    type: 'u64';
                },
                {
                    name: 'owner';
                    type: 'pubkey';
                },
                {
                    name: 'programOwner';
                    type: {
                        option: 'pubkey';
                    };
                },
                {
                    name: 'addressMerkleTreeConfig';
                    type: {
                        defined: {
                            name: 'addressMerkleTreeConfig';
                        };
                    };
                },
                {
                    name: 'addressQueueConfig';
                    type: {
                        defined: {
                            name: 'nullifierQueueConfig';
                        };
                    };
                },
            ];
        },
        {
            name: 'initializeGroupAuthority';
            docs: [
                'initialize group (a group can be used to give multiple programs access to the same Merkle trees by registering the programs to the group)',
            ];
            discriminator: [123, 237, 161, 80, 234, 215, 67, 183];
            accounts: [
                {
                    name: 'authority';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'seed';
                    docs: [
                        'Seed public key used to derive the group authority.',
                    ];
                    signer: true;
                },
                {
                    name: 'groupAuthority';
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'const';
                                value: [
                                    103,
                                    114,
                                    111,
                                    117,
                                    112,
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
                            {
                                kind: 'account';
                                path: 'seed';
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
            ];
        },
        {
            name: 'initializeStateMerkleTreeAndNullifierQueue';
            docs: [
                'Initializes a new Merkle tree from config bytes.',
                'Index is an optional identifier and not checked by the program.',
            ];
            discriminator: [45, 191, 235, 231, 63, 209, 142, 148];
            accounts: [
                {
                    name: 'authority';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'merkleTree';
                    writable: true;
                },
                {
                    name: 'nullifierQueue';
                    writable: true;
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
            ];
            args: [
                {
                    name: 'index';
                    type: 'u64';
                },
                {
                    name: 'owner';
                    type: 'pubkey';
                },
                {
                    name: 'programOwner';
                    type: {
                        option: 'pubkey';
                    };
                },
                {
                    name: 'stateMerkleTreeConfig';
                    type: {
                        defined: {
                            name: 'stateMerkleTreeConfig';
                        };
                    };
                },
                {
                    name: 'nullifierQueueConfig';
                    type: {
                        defined: {
                            name: 'nullifierQueueConfig';
                        };
                    };
                },
                {
                    name: 'additionalRent';
                    type: 'u64';
                },
            ];
        },
        {
            name: 'insertAddresses';
            discriminator: [248, 232, 179, 199, 27, 62, 56, 20];
            accounts: [
                {
                    name: 'feePayer';
                    docs: ['Fee payer pays rollover fee.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'registeredProgramPda';
                    optional: true;
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
            ];
            args: [
                {
                    name: 'addresses';
                    type: {
                        vec: {
                            array: ['u8', 32];
                        };
                    };
                },
            ];
        },
        {
            name: 'insertIntoNullifierQueues';
            discriminator: [91, 101, 183, 28, 35, 25, 67, 221];
            accounts: [
                {
                    name: 'feePayer';
                    docs: ['Fee payer pays rollover fee.'];
                    writable: true;
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'registeredProgramPda';
                    optional: true;
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
            ];
            args: [
                {
                    name: 'nullifiers';
                    type: {
                        vec: {
                            array: ['u8', 32];
                        };
                    };
                },
            ];
        },
        {
            name: 'nullifyLeaves';
            discriminator: [158, 91, 21, 224, 159, 65, 177, 67];
            accounts: [
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'registeredProgramPda';
                    optional: true;
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
            name: 'registerProgramToGroup';
            discriminator: [225, 86, 207, 211, 21, 1, 46, 25];
            accounts: [
                {
                    name: 'authority';
                    writable: true;
                    signer: true;
                },
                {
                    name: 'programToBeRegistered';
                    signer: true;
                },
                {
                    name: 'registeredProgramPda';
                    writable: true;
                    pda: {
                        seeds: [
                            {
                                kind: 'account';
                                path: 'programToBeRegistered';
                            },
                        ];
                    };
                },
                {
                    name: 'groupAuthorityPda';
                },
                {
                    name: 'systemProgram';
                    address: '11111111111111111111111111111111';
                },
            ];
            args: [];
        },
        {
            name: 'rolloverAddressMerkleTreeAndQueue';
            discriminator: [24, 84, 27, 12, 134, 166, 23, 192];
            accounts: [
                {
                    name: 'feePayer';
                    docs: [
                        'Signer used to receive rollover accounts rentexemption reimbursement.',
                    ];
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'registeredProgramPda';
                    optional: true;
                },
                {
                    name: 'newAddressMerkleTree';
                    writable: true;
                },
                {
                    name: 'newQueue';
                    writable: true;
                },
                {
                    name: 'oldAddressMerkleTree';
                    writable: true;
                },
                {
                    name: 'oldQueue';
                    writable: true;
                },
            ];
            args: [];
        },
        {
            name: 'rolloverStateMerkleTreeAndNullifierQueue';
            discriminator: [39, 161, 103, 172, 102, 198, 120, 85];
            accounts: [
                {
                    name: 'feePayer';
                    docs: [
                        'Signer used to receive rollover accounts rentexemption reimbursement.',
                    ];
                    signer: true;
                },
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'registeredProgramPda';
                    optional: true;
                },
                {
                    name: 'newStateMerkleTree';
                    writable: true;
                },
                {
                    name: 'newNullifierQueue';
                    writable: true;
                },
                {
                    name: 'oldStateMerkleTree';
                    writable: true;
                },
                {
                    name: 'oldNullifierQueue';
                    writable: true;
                },
            ];
            args: [];
        },
        {
            name: 'updateAddressMerkleTree';
            docs: ['Updates the address Merkle tree with a new address.'];
            discriminator: [75, 208, 63, 56, 207, 74, 124, 18];
            accounts: [
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'registeredProgramPda';
                    optional: true;
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
            name: 'updateGroupAuthority';
            discriminator: [113, 193, 181, 28, 214, 157, 178, 131];
            accounts: [
                {
                    name: 'authority';
                    signer: true;
                },
                {
                    name: 'groupAuthority';
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
    ];
    accounts: [
        {
            name: 'addressMerkleTreeAccount';
            discriminator: [11, 161, 175, 9, 212, 229, 73, 73];
        },
        {
            name: 'groupAuthority';
            discriminator: [15, 207, 4, 160, 127, 38, 142, 162];
        },
        {
            name: 'queueAccount';
            discriminator: [164, 200, 108, 62, 87, 63, 123, 65];
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
            name: 'integerOverflow';
            msg: 'Integer overflow';
        },
        {
            code: 6001;
            name: 'invalidAuthority';
            msg: 'invalidAuthority';
        },
        {
            code: 6002;
            name: 'invalidVerifier';
            msg: 'invalidVerifier';
        },
        {
            code: 6003;
            name: 'numberOfLeavesMismatch';
            msg: 'Leaves <> remaining accounts mismatch. The number of remaining accounts must match the number of leaves.';
        },
        {
            code: 6004;
            name: 'invalidNoopPubkey';
            msg: 'Provided noop program public key is invalid';
        },
        {
            code: 6005;
            name: 'numberOfChangeLogIndicesMismatch';
            msg: 'Number of change log indices mismatch';
        },
        {
            code: 6006;
            name: 'numberOfIndicesMismatch';
            msg: 'Number of indices mismatch';
        },
        {
            code: 6007;
            name: 'numberOfProofsMismatch';
            msg: 'numberOfProofsMismatch';
        },
        {
            code: 6008;
            name: 'invalidMerkleProof';
            msg: 'invalidMerkleProof';
        },
        {
            code: 6009;
            name: 'invalidMerkleTree';
            msg: 'invalidMerkleTree';
        },
        {
            code: 6010;
            name: 'leafNotFound';
            msg: 'Could not find the leaf in the queue';
        },
        {
            code: 6011;
            name: 'merkleTreeAndQueueNotAssociated';
            msg: 'merkleTreeAndQueueNotAssociated';
        },
        {
            code: 6012;
            name: 'merkleTreeAlreadyRolledOver';
            msg: 'merkleTreeAlreadyRolledOver';
        },
        {
            code: 6013;
            name: 'notReadyForRollover';
            msg: 'notReadyForRollover';
        },
        {
            code: 6014;
            name: 'rolloverNotConfigured';
            msg: 'rolloverNotConfigured';
        },
        {
            code: 6015;
            name: 'notAllLeavesProcessed';
            msg: 'notAllLeavesProcessed';
        },
        {
            code: 6016;
            name: 'invalidQueueType';
            msg: 'invalidQueueType';
        },
        {
            code: 6017;
            name: 'inputElementsEmpty';
            msg: 'inputElementsEmpty';
        },
        {
            code: 6018;
            name: 'noLeavesForMerkleTree';
            msg: 'noLeavesForMerkleTree';
        },
        {
            code: 6019;
            name: 'sizeMismatch';
            msg: 'sizeMismatch';
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
            name: 'queueAccount';
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
                                name: 'queueMetadata';
                            };
                        };
                    },
                ];
            };
        },
        {
            name: 'queueMetadata';
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
                        name: 'associatedMerkleTree';
                        type: 'pubkey';
                    },
                    {
                        name: 'nextQueue';
                        type: 'pubkey';
                    },
                    {
                        name: 'queueType';
                        type: 'u64';
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
            name: 'addressMerkleTreeCanopyDepth';
            type: 'u64';
            value: '10';
        },
        {
            name: 'addressMerkleTreeChangelog';
            type: 'u64';
            value: '1400';
        },
        {
            name: 'addressMerkleTreeHeight';
            type: 'u64';
            value: '26';
        },
        {
            name: 'addressMerkleTreeIndexedChangelog';
            type: 'u64';
            value: '256';
        },
        {
            name: 'addressMerkleTreeRoots';
            type: 'u64';
            value: '2400';
        },
        {
            name: 'addressQueueSequenceThreshold';
            type: 'u64';
            value: '2400';
        },
        {
            name: 'addressQueueValues';
            type: 'u16';
            value: '6857';
        },
        {
            name: 'cpiAuthorityPdaSeed';
            type: 'bytes';
            value: '[99, 112, 105, 95, 97, 117, 116, 104, 111, 114, 105, 116, 121]';
        },
        {
            name: 'groupAuthoritySeed';
            type: 'bytes';
            value: '[103, 114, 111, 117, 112, 95, 97, 117, 116, 104, 111, 114, 105, 116, 121]';
        },
        {
            name: 'noopPubkey';
            type: {
                array: ['u8', 32];
            };
            value: '[11, 188, 15, 192, 187, 71, 202, 47, 116, 196, 17, 46, 148, 171, 19, 207, 163, 198, 52, 229, 220, 23, 234, 203, 3, 205, 26, 35, 205, 126, 120, 124]';
        },
        {
            name: 'stateMerkleTreeCanopyDepth';
            type: 'u64';
            value: '10';
        },
        {
            name: 'stateMerkleTreeChangelog';
            type: 'u64';
            value: '1400';
        },
        {
            name: 'stateMerkleTreeHeight';
            type: 'u64';
            value: '26';
        },
        {
            name: 'stateMerkleTreeRoots';
            type: 'u64';
            value: '2400';
        },
        {
            name: 'stateNullifierQueueSequenceThreshold';
            type: 'u64';
            value: '2400';
        },
        {
            name: 'stateNullifierQueueValues';
            type: 'u16';
            value: '6857';
        },
    ];
};
