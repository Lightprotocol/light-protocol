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
      type: {
        defined: 'usize';
      };
      value: '26';
    },
    {
      name: 'STATE_MERKLE_TREE_CHANGELOG';
      type: {
        defined: 'usize';
      };
      value: '1400';
    },
    {
      name: 'STATE_MERKLE_TREE_ROOTS';
      type: {
        defined: 'usize';
      };
      value: '2400';
    },
    {
      name: 'STATE_INDEXED_ARRAY_SIZE';
      type: {
        defined: 'usize';
      };
      value: '4800';
    },
    {
      name: 'ADDRESS_MERKLE_TREE_HEIGHT';
      type: {
        defined: 'usize';
      };
      value: '22';
    },
    {
      name: 'ADDRESS_MERKLE_TREE_CHANGELOG';
      type: {
        defined: 'usize';
      };
      value: '2800';
    },
    {
      name: 'ADDRESS_MERKLE_TREE_ROOTS';
      type: {
        defined: 'usize';
      };
      value: '2800';
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
      args: [];
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
          name: 'queue';
          isMut: true;
          isSigner: false;
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
          name: 'queueIndex';
          type: 'u16';
        },
        {
          name: 'addressNextIndex';
          type: {
            defined: 'usize';
          };
        },
        {
          name: 'addressNextValue';
          type: {
            array: ['u8', 32];
          };
        },
        {
          name: 'lowAddress';
          type: {
            defined: 'RawIndexingElement<usize,32>';
          };
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
              22,
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
        'Can only be called from the merkle_tree_authority.',
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
      ];
    },
    {
      name: 'insertLeavesIntoMerkleTrees';
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
            name: 'array';
            type: 'publicKey';
          },
          {
            name: 'indexedArray';
            type: {
              array: ['u8', 192008];
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
            name: 'queue';
            type: {
              array: ['u8', 112008];
            };
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
              array: ['u8', 224];
            };
          },
          {
            name: 'merkleTreeFilledSubtrees';
            type: {
              array: ['u8', 704];
            };
          },
          {
            name: 'merkleTreeChangelog';
            type: {
              array: ['u8', 2083200];
            };
          },
          {
            name: 'merkleTreeRoots';
            type: {
              array: ['u8', 89600];
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
            name: 'stateMerkleTreeStruct';
            docs: ['Merkle tree for the transaction state.'];
            type: {
              array: ['u8', 224];
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
        ];
      };
    },
  ];
  types: [
    {
      name: 'Changelogs';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'changelogs';
            type: {
              vec: {
                defined: 'ChangelogEvent';
              };
            };
          },
        ];
      };
    },
    {
      name: 'PathNode';
      docs: [
        'Node of the Merkle path with an index representing the position in a',
        'non-sparse Merkle tree.',
      ];
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'node';
            type: {
              array: ['u8', 32];
            };
          },
          {
            name: 'index';
            type: 'u32';
          },
        ];
      };
    },
    {
      name: 'ChangelogEventV1';
      docs: [
        'Version 1 of the [`ChangelogEvent`](light_merkle_tree_program::state::ChangelogEvent).',
      ];
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'id';
            docs: ['Public key of the tree.'];
            type: 'publicKey';
          },
          {
            name: 'paths';
            type: {
              vec: {
                vec: {
                  defined: 'PathNode';
                };
              };
            };
          },
          {
            name: 'seq';
            docs: ['Number of successful operations on the on-chain tree.'];
            type: 'u64';
          },
          {
            name: 'index';
            docs: ['Changelog event index.'];
            type: 'u32';
          },
        ];
      };
    },
    {
      name: 'ChangelogEvent';
      docs: [
        'Event containing the Merkle path of the given',
        '[`StateMerkleTree`](light_merkle_tree_program::state::StateMerkleTree)',
        'change. Indexers can use this type of events to re-build a non-sparse',
        'version of state Merkle tree.',
      ];
      type: {
        kind: 'enum';
        variants: [
          {
            name: 'V1';
            fields: [
              {
                defined: 'ChangelogEventV1';
              },
            ];
          },
        ];
      };
    },
    {
      name: 'IndexedArray';
      type: {
        kind: 'alias';
        value: {
          defined: 'IndexingArray<Poseidon,u16,BigInteger256,STATE_INDEXED_ARRAY_SIZE>';
        };
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
  ];
};

export const IDL: AccountCompression = {
  version: '0.3.1',
  name: 'account_compression',
  constants: [
    {
      name: 'GROUP_AUTHORITY_SEED',
      type: 'bytes',
      value:
        '[103, 114, 111, 117, 112, 95, 97, 117, 116, 104, 111, 114, 105, 116, 121]',
    },
    {
      name: 'STATE_MERKLE_TREE_HEIGHT',
      type: {
        defined: 'usize',
      },
      value: '26',
    },
    {
      name: 'STATE_MERKLE_TREE_CHANGELOG',
      type: {
        defined: 'usize',
      },
      value: '1400',
    },
    {
      name: 'STATE_MERKLE_TREE_ROOTS',
      type: {
        defined: 'usize',
      },
      value: '2400',
    },
    {
      name: 'STATE_INDEXED_ARRAY_SIZE',
      type: {
        defined: 'usize',
      },
      value: '4800',
    },
    {
      name: 'ADDRESS_MERKLE_TREE_HEIGHT',
      type: {
        defined: 'usize',
      },
      value: '22',
    },
    {
      name: 'ADDRESS_MERKLE_TREE_CHANGELOG',
      type: {
        defined: 'usize',
      },
      value: '2800',
    },
    {
      name: 'ADDRESS_MERKLE_TREE_ROOTS',
      type: {
        defined: 'usize',
      },
      value: '2800',
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
      args: [],
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
          name: 'queue',
          isMut: true,
          isSigner: false,
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
          name: 'queueIndex',
          type: 'u16',
        },
        {
          name: 'addressNextIndex',
          type: {
            defined: 'usize',
          },
        },
        {
          name: 'addressNextValue',
          type: {
            array: ['u8', 32],
          },
        },
        {
          name: 'lowAddress',
          type: {
            defined: 'RawIndexingElement<usize,32>',
          },
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
              22,
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
        'Can only be called from the merkle_tree_authority.',
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
      ],
    },
    {
      name: 'insertLeavesIntoMerkleTrees',
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
            name: 'array',
            type: 'publicKey',
          },
          {
            name: 'indexedArray',
            type: {
              array: ['u8', 192008],
            },
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
            name: 'queue',
            type: {
              array: ['u8', 112008],
            },
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
              array: ['u8', 224],
            },
          },
          {
            name: 'merkleTreeFilledSubtrees',
            type: {
              array: ['u8', 704],
            },
          },
          {
            name: 'merkleTreeChangelog',
            type: {
              array: ['u8', 2083200],
            },
          },
          {
            name: 'merkleTreeRoots',
            type: {
              array: ['u8', 89600],
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
            name: 'stateMerkleTreeStruct',
            docs: ['Merkle tree for the transaction state.'],
            type: {
              array: ['u8', 224],
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
        ],
      },
    },
  ],
  types: [
    {
      name: 'Changelogs',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'changelogs',
            type: {
              vec: {
                defined: 'ChangelogEvent',
              },
            },
          },
        ],
      },
    },
    {
      name: 'PathNode',
      docs: [
        'Node of the Merkle path with an index representing the position in a',
        'non-sparse Merkle tree.',
      ],
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'node',
            type: {
              array: ['u8', 32],
            },
          },
          {
            name: 'index',
            type: 'u32',
          },
        ],
      },
    },
    {
      name: 'ChangelogEventV1',
      docs: [
        'Version 1 of the [`ChangelogEvent`](light_merkle_tree_program::state::ChangelogEvent).',
      ],
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'id',
            docs: ['Public key of the tree.'],
            type: 'publicKey',
          },
          {
            name: 'paths',
            type: {
              vec: {
                vec: {
                  defined: 'PathNode',
                },
              },
            },
          },
          {
            name: 'seq',
            docs: ['Number of successful operations on the on-chain tree.'],
            type: 'u64',
          },
          {
            name: 'index',
            docs: ['Changelog event index.'],
            type: 'u32',
          },
        ],
      },
    },
    {
      name: 'ChangelogEvent',
      docs: [
        'Event containing the Merkle path of the given',
        '[`StateMerkleTree`](light_merkle_tree_program::state::StateMerkleTree)',
        'change. Indexers can use this type of events to re-build a non-sparse',
        'version of state Merkle tree.',
      ],
      type: {
        kind: 'enum',
        variants: [
          {
            name: 'V1',
            fields: [
              {
                defined: 'ChangelogEventV1',
              },
            ],
          },
        ],
      },
    },
    {
      name: 'IndexedArray',
      type: {
        kind: 'alias',
        value: {
          defined:
            'IndexingArray<Poseidon,u16,BigInteger256,STATE_INDEXED_ARRAY_SIZE>',
        },
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
  ],
};