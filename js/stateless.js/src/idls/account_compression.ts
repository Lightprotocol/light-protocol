export type AccountCompression = {
    version: '0.3.1';
    name: 'account_compression';
    constants: [
        {
            name: 'GROUP_AUTHORITY_SEED';
            type: 'bytes';
            value: '[103, 114, 111, 117, 112, 95, 97, 117, 116, 104, 111, 114, 105, 116, 121]';
        },
        {
            name: 'STATE_MERKLE_TREE_HEIGHT';
            type: 'u64';
            value: '26';
        },
        {
            name: 'STATE_MERKLE_TREE_CHANGELOG';
            type: 'u64';
            value: '1400';
        },
        {
            name: 'STATE_MERKLE_TREE_ROOTS';
            type: 'u64';
            value: '2400';
        },
        {
            name: 'STATE_MERKLE_TREE_CANOPY_DEPTH';
            type: 'u64';
            value: '10';
        },
        {
            name: 'STATE_INDEXED_ARRAY_INDICES';
            type: 'u16';
            value: '6857';
        },
        {
            name: 'STATE_INDEXED_ARRAY_VALUES';
            type: 'u16';
            value: '4800';
        },
        {
            name: 'STATE_INDEXED_ARRAY_SEQUENCE_THRESHOLD';
            type: 'u64';
            value: '2400';
        },
        {
            name: 'ADDRESS_MERKLE_TREE_HEIGHT';
            type: 'u64';
            value: '26';
        },
        {
            name: 'ADDRESS_MERKLE_TREE_CHANGELOG';
            type: 'u64';
            value: '1400';
        },
        {
            name: 'ADDRESS_MERKLE_TREE_ROOTS';
            type: 'u64';
            value: '2400';
        },
        {
            name: 'ADDRESS_MERKLE_TREE_CANOPY_DEPTH';
            type: 'u64';
            value: '10';
        },
        {
            name: 'ADDRESS_QUEUE_INDICES';
            type: 'u16';
            value: '6857';
        },
        {
            name: 'ADDRESS_QUEUE_VALUES';
            type: 'u16';
            value: '4800';
        },
        {
            name: 'ADDRESS_QUEUE_SEQUENCE_THRESHOLD';
            type: 'u64';
            value: '2400';
        },
        {
            name: 'PROGRAM_ID';
            type: 'string';
            value: '"5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN"';
        },
    ];
    instructions: [
        {
            name: 'initializeAddressQueue';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'queue';
                    isMut: true;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'index';
                    type: 'u64';
                },
                {
                    name: 'owner';
                    type: 'publicKey';
                },
                {
                    name: 'delegate';
                    type: {
                        option: 'publicKey';
                    };
                },
                {
                    name: 'associatedMerkleTree';
                    type: {
                        option: 'publicKey';
                    };
                },
                {
                    name: 'capacityIndices';
                    type: 'u16';
                },
                {
                    name: 'capacityValues';
                    type: 'u16';
                },
                {
                    name: 'sequenceThreshold';
                    type: 'u64';
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
                },
                {
                    name: 'merkleTree';
                    isMut: true;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'index';
                    type: 'u64';
                },
                {
                    name: 'owner';
                    type: 'publicKey';
                },
                {
                    name: 'delegate';
                    type: {
                        option: 'publicKey';
                    };
                },
                {
                    name: 'height';
                    type: 'u64';
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
            ];
        },
        {
            name: 'insertAddresses';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                    isOptional: true;
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
            name: 'updateAddressMerkleTree';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
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
            ];
            args: [
                {
                    name: 'changelogIndex';
                    type: 'u16';
                },
                {
                    name: 'value';
                    type: {
                        array: ['u8', 32];
                    };
                },
                {
                    name: 'nextIndex';
                    type: 'u64';
                },
                {
                    name: 'nextValue';
                    type: {
                        array: ['u8', 32];
                    };
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
                {
                    name: 'nextAddressProof';
                    type: {
                        array: ['u8', 128];
                    };
                },
            ];
        },
        {
            name: 'initializeGroupAuthority';
            docs: [
                'initialize group (a group can be used to give multiple programs acess to the same Merkle trees by registering the programs to the group)',
            ];
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'groupAuthority';
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
                    name: 'seed';
                    type: {
                        array: ['u8', 32];
                    };
                },
                {
                    name: 'authority';
                    type: 'publicKey';
                },
            ];
        },
        {
            name: 'updateGroupAuthority';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'groupAuthority';
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
            name: 'registerProgramToGroup';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'groupAuthorityPda';
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
                    name: 'programId';
                    type: 'publicKey';
                },
            ];
        },
        {
            name: 'initializeStateMerkleTree';
            docs: [
                'Initializes a new Merkle tree from config bytes.',
                'Index is an optional identifier and not checked by the program.',
                'TODO: think the index over',
            ];
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'merkleTree';
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
                    name: 'index';
                    type: 'u64';
                },
                {
                    name: 'owner';
                    type: 'publicKey';
                },
                {
                    name: 'delegate';
                    type: {
                        option: 'publicKey';
                    };
                },
                {
                    name: 'height';
                    type: 'u64';
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
                    name: 'associatedQueue';
                    type: {
                        option: 'publicKey';
                    };
                },
            ];
        },
        {
            name: 'appendLeavesToMerkleTrees';
            accounts: [
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                    isOptional: true;
                },
                {
                    name: 'logWrapper';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'leaves';
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
            accounts: [
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                    isOptional: true;
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
                    name: 'indexedArray';
                    isMut: true;
                    isSigner: false;
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
            name: 'initializeIndexedArray';
            accounts: [
                {
                    name: 'authority';
                    isMut: false;
                    isSigner: true;
                },
                {
                    name: 'indexedArray';
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
                    name: 'index';
                    type: 'u64';
                },
                {
                    name: 'owner';
                    type: 'publicKey';
                },
                {
                    name: 'delegate';
                    type: {
                        option: 'publicKey';
                    };
                },
                {
                    name: 'associatedMerkleTree';
                    type: {
                        option: 'publicKey';
                    };
                },
                {
                    name: 'capacityIndices';
                    type: 'u16';
                },
                {
                    name: 'capacityValues';
                    type: 'u16';
                },
                {
                    name: 'sequenceThreshold';
                    type: 'u64';
                },
            ];
        },
        {
            name: 'insertIntoIndexedArrays';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                    isOptional: true;
                },
            ];
            args: [
                {
                    name: 'elements';
                    type: {
                        vec: {
                            array: ['u8', 32];
                        };
                    };
                },
            ];
        },
    ];
    accounts: [
        {
            name: 'groupAuthority';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'publicKey';
                    },
                    {
                        name: 'seed';
                        type: {
                            array: ['u8', 32];
                        };
                    },
                ];
            };
        },
        {
            name: 'indexedArrayAccount';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'index';
                        type: 'u64';
                    },
                    {
                        name: 'owner';
                        type: 'publicKey';
                    },
                    {
                        name: 'delegate';
                        type: 'publicKey';
                    },
                    {
                        name: 'associatedMerkleTree';
                        type: 'publicKey';
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
                        name: 'pubkey';
                        type: 'publicKey';
                    },
                ];
            };
        },
        {
            name: 'addressQueueAccount';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'index';
                        type: 'u64';
                    },
                    {
                        name: 'owner';
                        type: 'publicKey';
                    },
                    {
                        name: 'delegate';
                        type: 'publicKey';
                    },
                    {
                        name: 'associatedMerkleTree';
                        type: 'publicKey';
                    },
                ];
            };
        },
        {
            name: 'addressMerkleTreeAccount';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'index';
                        docs: ['Unique index.'];
                        type: 'u64';
                    },
                    {
                        name: 'nextMerkleTree';
                        docs: ['Public key of the next Merkle tree.'];
                        type: 'publicKey';
                    },
                    {
                        name: 'owner';
                        docs: ['Owner of the Merkle tree.'];
                        type: 'publicKey';
                    },
                    {
                        name: 'delegate';
                        docs: [
                            'Delegate of the Merkle tree. This will be used for program owned Merkle trees.',
                        ];
                        type: 'publicKey';
                    },
                    {
                        name: 'merkleTreeStruct';
                        type: {
                            array: ['u8', 256];
                        };
                    },
                    {
                        name: 'merkleTreeFilledSubtrees';
                        type: {
                            array: ['u8', 832];
                        };
                    },
                    {
                        name: 'merkleTreeChangelog';
                        type: {
                            array: ['u8', 1220800];
                        };
                    },
                    {
                        name: 'merkleTreeRoots';
                        type: {
                            array: ['u8', 76800];
                        };
                    },
                    {
                        name: 'merkleTreeCanopy';
                        type: {
                            array: ['u8', 65472];
                        };
                    },
                ];
            };
        },
        {
            name: 'stateMerkleTreeAccount';
            docs: [
                'Concurrent state Merkle tree used for public compressed transactions.',
            ];
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'index';
                        docs: ['Unique index.'];
                        type: 'u64';
                    },
                    {
                        name: 'nextMerkleTree';
                        docs: ['Public key of the next Merkle tree.'];
                        type: 'publicKey';
                    },
                    {
                        name: 'owner';
                        docs: ['Owner of the Merkle tree.'];
                        type: 'publicKey';
                    },
                    {
                        name: 'delegate';
                        docs: [
                            'Delegate of the Merkle tree. This will be used for program owned Merkle trees.',
                        ];
                        type: 'publicKey';
                    },
                    {
                        name: 'associatedQueue';
                        type: 'publicKey';
                    },
                    {
                        name: 'stateMerkleTreeStruct';
                        docs: ['Merkle tree for the transaction state.'];
                        type: {
                            array: ['u8', 256];
                        };
                    },
                    {
                        name: 'stateMerkleTreeFilledSubtrees';
                        type: {
                            array: ['u8', 832];
                        };
                    },
                    {
                        name: 'stateMerkleTreeChangelog';
                        type: {
                            array: ['u8', 1220800];
                        };
                    },
                    {
                        name: 'stateMerkleTreeRoots';
                        type: {
                            array: ['u8', 76800];
                        };
                    },
                    {
                        name: 'stateMerkleTreeCanopy';
                        type: {
                            array: ['u8', 65472];
                        };
                    },
                ];
            };
        },
    ];
    errors: [
        {
            code: 6000;
            name: 'AddressQueueInsert';
            msg: 'Failed to insert an element into indexing queue';
        },
        {
            code: 6001;
            name: 'AddressQueueDequeue';
            msg: 'Failed to dequeue an element from indexing queue';
        },
        {
            code: 6002;
            name: 'AddressMerkleTreeInitialize';
            msg: 'Failed to initialize address Merkle tree';
        },
        {
            code: 6003;
            name: 'AddressMerkleTreeUpdate';
            msg: 'Failed to update the address Merkle tree';
        },
        {
            code: 6004;
            name: 'InvalidIndex';
            msg: 'No element found under the given index in the queue';
        },
        {
            code: 6005;
            name: 'BytesToBigint';
            msg: 'Failed to convert bytes to big integer';
        },
        {
            code: 6006;
            name: 'IntegerOverflow';
            msg: 'Integer overflow';
        },
        {
            code: 6007;
            name: 'InvalidAuthority';
            msg: 'InvalidAuthority';
        },
        {
            code: 6008;
            name: 'InvalidVerifier';
            msg: 'InvalidVerifier';
        },
        {
            code: 6009;
            name: 'NumberOfLeavesMismatch';
            msg: 'Leaves <> remaining accounts missmatch. The number of remaining accounts must match the number of leaves.';
        },
        {
            code: 6010;
            name: 'InvalidNoopPubkey';
            msg: 'Provided noop program public key is invalid';
        },
        {
            code: 6011;
            name: 'EventNoChangelogEntry';
            msg: 'Emitting an event requires at least one changelog entry';
        },
        {
            code: 6012;
            name: 'NumberOfChangeLogIndicesMismatch';
            msg: 'Number of change log indices mismatch';
        },
        {
            code: 6013;
            name: 'NumberOfIndicesMismatch';
            msg: 'Number of indices mismatch';
        },
        {
            code: 6014;
            name: 'IndexOutOfBounds';
            msg: 'IndexOutOfBounds';
        },
        {
            code: 6015;
            name: 'ElementAlreadyExists';
            msg: 'ElementAlreadyExists';
        },
        {
            code: 6016;
            name: 'HashSetFull';
            msg: 'HashSetFull';
        },
        {
            code: 6017;
            name: 'NumberOfProofsMismatch';
            msg: 'NumberOfProofsMismatch';
        },
        {
            code: 6018;
            name: 'InvalidMerkleProof';
            msg: 'InvalidMerkleProof';
        },
        {
            code: 6019;
            name: 'InvalidIndexedArray';
            msg: 'InvalidIndexedArray';
        },
        {
            code: 6020;
            name: 'InvalidMerkleTree';
            msg: 'InvalidMerkleTree';
        },
        {
            code: 6021;
            name: 'LeafNotFound';
            msg: 'Could not find the leaf in the queue';
        },
    ];
};

export const IDL: AccountCompression = {
    version: '0.3.1',
    name: 'account_compression',
    constants: [
        {
            name: 'GROUP_AUTHORITY_SEED',
            type: 'bytes',
            value: '[103, 114, 111, 117, 112, 95, 97, 117, 116, 104, 111, 114, 105, 116, 121]',
        },
        {
            name: 'STATE_MERKLE_TREE_HEIGHT',
            type: 'u64',
            value: '26',
        },
        {
            name: 'STATE_MERKLE_TREE_CHANGELOG',
            type: 'u64',
            value: '1400',
        },
        {
            name: 'STATE_MERKLE_TREE_ROOTS',
            type: 'u64',
            value: '2400',
        },
        {
            name: 'STATE_MERKLE_TREE_CANOPY_DEPTH',
            type: 'u64',
            value: '10',
        },
        {
            name: 'STATE_INDEXED_ARRAY_INDICES',
            type: 'u16',
            value: '6857',
        },
        {
            name: 'STATE_INDEXED_ARRAY_VALUES',
            type: 'u16',
            value: '4800',
        },
        {
            name: 'STATE_INDEXED_ARRAY_SEQUENCE_THRESHOLD',
            type: 'u64',
            value: '2400',
        },
        {
            name: 'ADDRESS_MERKLE_TREE_HEIGHT',
            type: 'u64',
            value: '26',
        },
        {
            name: 'ADDRESS_MERKLE_TREE_CHANGELOG',
            type: 'u64',
            value: '1400',
        },
        {
            name: 'ADDRESS_MERKLE_TREE_ROOTS',
            type: 'u64',
            value: '2400',
        },
        {
            name: 'ADDRESS_MERKLE_TREE_CANOPY_DEPTH',
            type: 'u64',
            value: '10',
        },
        {
            name: 'ADDRESS_QUEUE_INDICES',
            type: 'u16',
            value: '6857',
        },
        {
            name: 'ADDRESS_QUEUE_VALUES',
            type: 'u16',
            value: '4800',
        },
        {
            name: 'ADDRESS_QUEUE_SEQUENCE_THRESHOLD',
            type: 'u64',
            value: '2400',
        },
        {
            name: 'PROGRAM_ID',
            type: 'string',
            value: '"5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN"',
        },
    ],
    instructions: [
        {
            name: 'initializeAddressQueue',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'queue',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'index',
                    type: 'u64',
                },
                {
                    name: 'owner',
                    type: 'publicKey',
                },
                {
                    name: 'delegate',
                    type: {
                        option: 'publicKey',
                    },
                },
                {
                    name: 'associatedMerkleTree',
                    type: {
                        option: 'publicKey',
                    },
                },
                {
                    name: 'capacityIndices',
                    type: 'u16',
                },
                {
                    name: 'capacityValues',
                    type: 'u16',
                },
                {
                    name: 'sequenceThreshold',
                    type: 'u64',
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
                },
                {
                    name: 'merkleTree',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'index',
                    type: 'u64',
                },
                {
                    name: 'owner',
                    type: 'publicKey',
                },
                {
                    name: 'delegate',
                    type: {
                        option: 'publicKey',
                    },
                },
                {
                    name: 'height',
                    type: 'u64',
                },
                {
                    name: 'changelogSize',
                    type: 'u64',
                },
                {
                    name: 'rootsSize',
                    type: 'u64',
                },
                {
                    name: 'canopyDepth',
                    type: 'u64',
                },
            ],
        },
        {
            name: 'insertAddresses',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                    isOptional: true,
                },
            ],
            args: [
                {
                    name: 'addresses',
                    type: {
                        vec: {
                            array: ['u8', 32],
                        },
                    },
                },
            ],
        },
        {
            name: 'updateAddressMerkleTree',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
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
            ],
            args: [
                {
                    name: 'changelogIndex',
                    type: 'u16',
                },
                {
                    name: 'value',
                    type: {
                        array: ['u8', 32],
                    },
                },
                {
                    name: 'nextIndex',
                    type: 'u64',
                },
                {
                    name: 'nextValue',
                    type: {
                        array: ['u8', 32],
                    },
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
                {
                    name: 'nextAddressProof',
                    type: {
                        array: ['u8', 128],
                    },
                },
            ],
        },
        {
            name: 'initializeGroupAuthority',
            docs: [
                'initialize group (a group can be used to give multiple programs acess to the same Merkle trees by registering the programs to the group)',
            ],
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'groupAuthority',
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
                    name: 'seed',
                    type: {
                        array: ['u8', 32],
                    },
                },
                {
                    name: 'authority',
                    type: 'publicKey',
                },
            ],
        },
        {
            name: 'updateGroupAuthority',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'groupAuthority',
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
            name: 'registerProgramToGroup',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'groupAuthorityPda',
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
                    name: 'programId',
                    type: 'publicKey',
                },
            ],
        },
        {
            name: 'initializeStateMerkleTree',
            docs: [
                'Initializes a new Merkle tree from config bytes.',
                'Index is an optional identifier and not checked by the program.',
                'TODO: think the index over',
            ],
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'merkleTree',
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
                    name: 'index',
                    type: 'u64',
                },
                {
                    name: 'owner',
                    type: 'publicKey',
                },
                {
                    name: 'delegate',
                    type: {
                        option: 'publicKey',
                    },
                },
                {
                    name: 'height',
                    type: 'u64',
                },
                {
                    name: 'changelogSize',
                    type: 'u64',
                },
                {
                    name: 'rootsSize',
                    type: 'u64',
                },
                {
                    name: 'canopyDepth',
                    type: 'u64',
                },
                {
                    name: 'associatedQueue',
                    type: {
                        option: 'publicKey',
                    },
                },
            ],
        },
        {
            name: 'appendLeavesToMerkleTrees',
            accounts: [
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                    isOptional: true,
                },
                {
                    name: 'logWrapper',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'leaves',
                    type: {
                        vec: {
                            array: ['u8', 32],
                        },
                    },
                },
            ],
        },
        {
            name: 'nullifyLeaves',
            accounts: [
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                    isOptional: true,
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
                    name: 'indexedArray',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
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
            name: 'initializeIndexedArray',
            accounts: [
                {
                    name: 'authority',
                    isMut: false,
                    isSigner: true,
                },
                {
                    name: 'indexedArray',
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
                    name: 'index',
                    type: 'u64',
                },
                {
                    name: 'owner',
                    type: 'publicKey',
                },
                {
                    name: 'delegate',
                    type: {
                        option: 'publicKey',
                    },
                },
                {
                    name: 'associatedMerkleTree',
                    type: {
                        option: 'publicKey',
                    },
                },
                {
                    name: 'capacityIndices',
                    type: 'u16',
                },
                {
                    name: 'capacityValues',
                    type: 'u16',
                },
                {
                    name: 'sequenceThreshold',
                    type: 'u64',
                },
            ],
        },
        {
            name: 'insertIntoIndexedArrays',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                    isOptional: true,
                },
            ],
            args: [
                {
                    name: 'elements',
                    type: {
                        vec: {
                            array: ['u8', 32],
                        },
                    },
                },
            ],
        },
    ],
    accounts: [
        {
            name: 'groupAuthority',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'authority',
                        type: 'publicKey',
                    },
                    {
                        name: 'seed',
                        type: {
                            array: ['u8', 32],
                        },
                    },
                ],
            },
        },
        {
            name: 'indexedArrayAccount',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'index',
                        type: 'u64',
                    },
                    {
                        name: 'owner',
                        type: 'publicKey',
                    },
                    {
                        name: 'delegate',
                        type: 'publicKey',
                    },
                    {
                        name: 'associatedMerkleTree',
                        type: 'publicKey',
                    },
                ],
            },
        },
        {
            name: 'registeredProgram',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'pubkey',
                        type: 'publicKey',
                    },
                ],
            },
        },
        {
            name: 'addressQueueAccount',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'index',
                        type: 'u64',
                    },
                    {
                        name: 'owner',
                        type: 'publicKey',
                    },
                    {
                        name: 'delegate',
                        type: 'publicKey',
                    },
                    {
                        name: 'associatedMerkleTree',
                        type: 'publicKey',
                    },
                ],
            },
        },
        {
            name: 'addressMerkleTreeAccount',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'index',
                        docs: ['Unique index.'],
                        type: 'u64',
                    },
                    {
                        name: 'nextMerkleTree',
                        docs: ['Public key of the next Merkle tree.'],
                        type: 'publicKey',
                    },
                    {
                        name: 'owner',
                        docs: ['Owner of the Merkle tree.'],
                        type: 'publicKey',
                    },
                    {
                        name: 'delegate',
                        docs: [
                            'Delegate of the Merkle tree. This will be used for program owned Merkle trees.',
                        ],
                        type: 'publicKey',
                    },
                    {
                        name: 'merkleTreeStruct',
                        type: {
                            array: ['u8', 256],
                        },
                    },
                    {
                        name: 'merkleTreeFilledSubtrees',
                        type: {
                            array: ['u8', 832],
                        },
                    },
                    {
                        name: 'merkleTreeChangelog',
                        type: {
                            array: ['u8', 1220800],
                        },
                    },
                    {
                        name: 'merkleTreeRoots',
                        type: {
                            array: ['u8', 76800],
                        },
                    },
                    {
                        name: 'merkleTreeCanopy',
                        type: {
                            array: ['u8', 65472],
                        },
                    },
                ],
            },
        },
        {
            name: 'stateMerkleTreeAccount',
            docs: [
                'Concurrent state Merkle tree used for public compressed transactions.',
            ],
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'index',
                        docs: ['Unique index.'],
                        type: 'u64',
                    },
                    {
                        name: 'nextMerkleTree',
                        docs: ['Public key of the next Merkle tree.'],
                        type: 'publicKey',
                    },
                    {
                        name: 'owner',
                        docs: ['Owner of the Merkle tree.'],
                        type: 'publicKey',
                    },
                    {
                        name: 'delegate',
                        docs: [
                            'Delegate of the Merkle tree. This will be used for program owned Merkle trees.',
                        ],
                        type: 'publicKey',
                    },
                    {
                        name: 'associatedQueue',
                        type: 'publicKey',
                    },
                    {
                        name: 'stateMerkleTreeStruct',
                        docs: ['Merkle tree for the transaction state.'],
                        type: {
                            array: ['u8', 256],
                        },
                    },
                    {
                        name: 'stateMerkleTreeFilledSubtrees',
                        type: {
                            array: ['u8', 832],
                        },
                    },
                    {
                        name: 'stateMerkleTreeChangelog',
                        type: {
                            array: ['u8', 1220800],
                        },
                    },
                    {
                        name: 'stateMerkleTreeRoots',
                        type: {
                            array: ['u8', 76800],
                        },
                    },
                    {
                        name: 'stateMerkleTreeCanopy',
                        type: {
                            array: ['u8', 65472],
                        },
                    },
                ],
            },
        },
    ],
    errors: [
        {
            code: 6000,
            name: 'AddressQueueInsert',
            msg: 'Failed to insert an element into indexing queue',
        },
        {
            code: 6001,
            name: 'AddressQueueDequeue',
            msg: 'Failed to dequeue an element from indexing queue',
        },
        {
            code: 6002,
            name: 'AddressMerkleTreeInitialize',
            msg: 'Failed to initialize address Merkle tree',
        },
        {
            code: 6003,
            name: 'AddressMerkleTreeUpdate',
            msg: 'Failed to update the address Merkle tree',
        },
        {
            code: 6004,
            name: 'InvalidIndex',
            msg: 'No element found under the given index in the queue',
        },
        {
            code: 6005,
            name: 'BytesToBigint',
            msg: 'Failed to convert bytes to big integer',
        },
        {
            code: 6006,
            name: 'IntegerOverflow',
            msg: 'Integer overflow',
        },
        {
            code: 6007,
            name: 'InvalidAuthority',
            msg: 'InvalidAuthority',
        },
        {
            code: 6008,
            name: 'InvalidVerifier',
            msg: 'InvalidVerifier',
        },
        {
            code: 6009,
            name: 'NumberOfLeavesMismatch',
            msg: 'Leaves <> remaining accounts missmatch. The number of remaining accounts must match the number of leaves.',
        },
        {
            code: 6010,
            name: 'InvalidNoopPubkey',
            msg: 'Provided noop program public key is invalid',
        },
        {
            code: 6011,
            name: 'EventNoChangelogEntry',
            msg: 'Emitting an event requires at least one changelog entry',
        },
        {
            code: 6012,
            name: 'NumberOfChangeLogIndicesMismatch',
            msg: 'Number of change log indices mismatch',
        },
        {
            code: 6013,
            name: 'NumberOfIndicesMismatch',
            msg: 'Number of indices mismatch',
        },
        {
            code: 6014,
            name: 'IndexOutOfBounds',
            msg: 'IndexOutOfBounds',
        },
        {
            code: 6015,
            name: 'ElementAlreadyExists',
            msg: 'ElementAlreadyExists',
        },
        {
            code: 6016,
            name: 'HashSetFull',
            msg: 'HashSetFull',
        },
        {
            code: 6017,
            name: 'NumberOfProofsMismatch',
            msg: 'NumberOfProofsMismatch',
        },
        {
            code: 6018,
            name: 'InvalidMerkleProof',
            msg: 'InvalidMerkleProof',
        },
        {
            code: 6019,
            name: 'InvalidIndexedArray',
            msg: 'InvalidIndexedArray',
        },
        {
            code: 6020,
            name: 'InvalidMerkleTree',
            msg: 'InvalidMerkleTree',
        },
        {
            code: 6021,
            name: 'LeafNotFound',
            msg: 'Could not find the leaf in the queue',
        },
    ],
};
