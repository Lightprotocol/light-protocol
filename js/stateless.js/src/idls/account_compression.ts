export type AccountCompression = {
  "version": "0.3.5",
  "name": "account_compression",
  "constants": [
    {
      "name": "GROUP_AUTHORITY_SEED",
      "type": "bytes",
      "value": "[103, 114, 111, 117, 112, 95, 97, 117, 116, 104, 111, 114, 105, 116, 121]"
    },
    {
      "name": "STATE_MERKLE_TREE_HEIGHT",
      "type": "u64",
      "value": "26"
    },
    {
      "name": "STATE_MERKLE_TREE_CHANGELOG",
      "type": "u64",
      "value": "1400"
    },
    {
      "name": "STATE_MERKLE_TREE_ROOTS",
      "type": "u64",
      "value": "2400"
    },
    {
      "name": "STATE_MERKLE_TREE_CANOPY_DEPTH",
      "type": "u64",
      "value": "10"
    },
    {
      "name": "STATE_NULLIFIER_QUEUE_INDICES",
      "type": "u16",
      "value": "6857"
    },
    {
      "name": "STATE_NULLIFIER_QUEUE_VALUES",
      "type": "u16",
      "value": "4800"
    },
    {
      "name": "STATE_NULLIFIER_QUEUE_SEQUENCE_THRESHOLD",
      "type": "u64",
      "value": "2400"
    },
    {
      "name": "ADDRESS_MERKLE_TREE_HEIGHT",
      "type": "u64",
      "value": "26"
    },
    {
      "name": "ADDRESS_MERKLE_TREE_CHANGELOG",
      "type": "u64",
      "value": "1400"
    },
    {
      "name": "ADDRESS_MERKLE_TREE_ROOTS",
      "type": "u64",
      "value": "2400"
    },
    {
      "name": "ADDRESS_MERKLE_TREE_CANOPY_DEPTH",
      "type": "u64",
      "value": "10"
    },
    {
      "name": "ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG",
      "type": "u64",
      "value": "256"
    },
    {
      "name": "ADDRESS_QUEUE_INDICES",
      "type": "u16",
      "value": "6857"
    },
    {
      "name": "ADDRESS_QUEUE_VALUES",
      "type": "u16",
      "value": "4800"
    },
    {
      "name": "ADDRESS_QUEUE_SEQUENCE_THRESHOLD",
      "type": "u64",
      "value": "2400"
    },
    {
      "name": "NOOP_PUBKEY",
      "type": {
        "array": [
          "u8",
          32
        ]
      },
      "value": "[11 , 188 , 15 , 192 , 187 , 71 , 202 , 47 , 116 , 196 , 17 , 46 , 148 , 171 , 19 , 207 , 163 , 198 , 52 , 229 , 220 , 23 , 234 , 203 , 3 , 205 , 26 , 35 , 205 , 126 , 120 , 124 ,]"
    },
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN\""
    }
  ],
  "instructions": [
    {
      "name": "initializeAddressMerkleTreeAndQueue",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "queue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u64"
        },
        {
          "name": "owner",
          "type": "publicKey"
        },
        {
          "name": "delegate",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "addressMerkleTreeConfig",
          "type": {
            "defined": "AddressMerkleTreeConfig"
          }
        },
        {
          "name": "addressQueueConfig",
          "type": {
            "defined": "AddressQueueConfig"
          }
        }
      ]
    },
    {
      "name": "insertAddresses",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "addresses",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
          }
        }
      ]
    },
    {
      "name": "updateAddressMerkleTree",
      "docs": [
        "Updates the address Merkle tree with a new address."
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "queue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "logWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "changelogIndex",
          "type": "u16"
        },
        {
          "name": "value",
          "type": "u16"
        },
        {
          "name": "nextIndex",
          "type": "u64"
        },
        {
          "name": "lowAddressIndex",
          "type": "u64"
        },
        {
          "name": "lowAddressValue",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "lowAddressNextIndex",
          "type": "u64"
        },
        {
          "name": "lowAddressNextValue",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "lowAddressProof",
          "type": {
            "array": [
              {
                "array": [
                  "u8",
                  32
                ]
              },
              16
            ]
          }
        }
      ]
    },
    {
      "name": "rolloverAddressMerkleTreeAndQueue",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": false,
          "isSigner": true,
          "docs": [
            "Signer used to pay rollover and protocol fees."
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "newAddressMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "newQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oldAddressMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oldQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "initializeGroupAuthority",
      "docs": [
        "initialize group (a group can be used to give multiple programs acess to the same Merkle trees by registering the programs to the group)"
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "groupAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "seed",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "authority",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "updateGroupAuthority",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "groupAuthority",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "authority",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "registerProgramToGroup",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "groupAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "programId",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "initializeStateMerkleTreeAndNullifierQueue",
      "docs": [
        "Initializes a new Merkle tree from config bytes.",
        "Index is an optional identifier and not checked by the program."
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "nullifierQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u64"
        },
        {
          "name": "owner",
          "type": "publicKey"
        },
        {
          "name": "delegate",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "stateMerkleTreeConfig",
          "type": {
            "defined": "StateMerkleTreeConfig"
          }
        },
        {
          "name": "nullifierQueueConfig",
          "type": {
            "defined": "NullifierQueueConfig"
          }
        }
      ]
    },
    {
      "name": "appendLeavesToMerkleTrees",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Signer used to pay rollover and protocol fees."
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "logWrapper",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "leaves",
          "type": {
            "vec": {
              "defined": "(u8,[u8;32])"
            }
          }
        }
      ]
    },
    {
      "name": "nullifyLeaves",
      "accounts": [
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "logWrapper",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "nullifierQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "changeLogIndices",
          "type": {
            "vec": "u64"
          }
        },
        {
          "name": "leavesQueueIndices",
          "type": {
            "vec": "u16"
          }
        },
        {
          "name": "indices",
          "type": {
            "vec": "u64"
          }
        },
        {
          "name": "proofs",
          "type": {
            "vec": {
              "vec": {
                "array": [
                  "u8",
                  32
                ]
              }
            }
          }
        }
      ]
    },
    {
      "name": "insertIntoNullifierQueues",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "elements",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
          }
        }
      ]
    },
    {
      "name": "rolloverStateMerkleTreeAndNullifierQueue",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": false,
          "isSigner": true,
          "docs": [
            "Signer used to pay rollover and protocol fees."
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "newStateMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "newNullifierQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oldStateMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oldNullifierQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "groupAuthority",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "publicKey"
          },
          {
            "name": "seed",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "registeredProgram",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pubkey",
            "type": "publicKey"
          }
        ]
      }
    },
    {
      "name": "nullifierQueueAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "metadata",
            "type": {
              "defined": "QueueMetadata"
            }
          }
        ]
      }
    },
    {
      "name": "accessMetadata",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "docs": [
              "Owner of the Merkle tree."
            ],
            "type": "publicKey"
          },
          {
            "name": "delegate",
            "docs": [
              "Delegate of the Merkle tree. This will be used for program owned Merkle trees."
            ],
            "type": "publicKey"
          }
        ]
      }
    },
    {
      "name": "addressQueueAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "metadata",
            "type": {
              "defined": "QueueMetadata"
            }
          }
        ]
      }
    },
    {
      "name": "addressMerkleTreeAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "metadata",
            "type": {
              "defined": "MerkleTreeMetadata"
            }
          },
          {
            "name": "merkleTreeStruct",
            "type": {
              "array": [
                "u8",
                320
              ]
            }
          },
          {
            "name": "merkleTreeFilledSubtrees",
            "type": {
              "array": [
                "u8",
                832
              ]
            }
          },
          {
            "name": "merkleTreeChangelog",
            "type": {
              "array": [
                "u8",
                1220800
              ]
            }
          },
          {
            "name": "merkleTreeRoots",
            "type": {
              "array": [
                "u8",
                76800
              ]
            }
          },
          {
            "name": "merkleTreeCanopy",
            "type": {
              "array": [
                "u8",
                65472
              ]
            }
          },
          {
            "name": "addressChangelog",
            "type": {
              "array": [
                "u8",
                20480
              ]
            }
          }
        ]
      }
    },
    {
      "name": "merkleTreeMetadata",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "accessMetadata",
            "type": {
              "defined": "AccessMetadata"
            }
          },
          {
            "name": "rolloverMetadata",
            "type": {
              "defined": "RolloverMetadata"
            }
          },
          {
            "name": "associatedQueue",
            "type": "publicKey"
          },
          {
            "name": "nextMerkleTree",
            "type": "publicKey"
          }
        ]
      }
    },
    {
      "name": "stateMerkleTreeAccount",
      "docs": [
        "Concurrent state Merkle tree used for public compressed transactions."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "metadata",
            "type": {
              "defined": "MerkleTreeMetadata"
            }
          },
          {
            "name": "stateMerkleTreeStruct",
            "docs": [
              "Merkle tree for the transaction state."
            ],
            "type": {
              "array": [
                "u8",
                272
              ]
            }
          },
          {
            "name": "stateMerkleTreeFilledSubtrees",
            "type": {
              "array": [
                "u8",
                832
              ]
            }
          },
          {
            "name": "stateMerkleTreeChangelog",
            "type": {
              "array": [
                "u8",
                1220800
              ]
            }
          },
          {
            "name": "stateMerkleTreeRoots",
            "type": {
              "array": [
                "u8",
                76800
              ]
            }
          },
          {
            "name": "stateMerkleTreeCanopy",
            "type": {
              "array": [
                "u8",
                65472
              ]
            }
          }
        ]
      }
    },
    {
      "name": "queueMetadata",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "accessMetadata",
            "type": {
              "defined": "AccessMetadata"
            }
          },
          {
            "name": "rolloverMetadata",
            "type": {
              "defined": "RolloverMetadata"
            }
          },
          {
            "name": "associatedMerkleTree",
            "type": "publicKey"
          },
          {
            "name": "nextQueue",
            "type": "publicKey"
          }
        ]
      }
    },
    {
      "name": "rolloverMetadata",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "index",
            "docs": [
              "Unique index."
            ],
            "type": "u64"
          },
          {
            "name": "rolloverFee",
            "docs": [
              "This fee is used for rent for the next account.",
              "It accumulates in the account so that once the corresponding Merkle tree account is full it can be rolled over"
            ],
            "type": "u64"
          },
          {
            "name": "rolloverThreshold",
            "docs": [
              "The threshold in percentage points when the account should be rolled over (95 corresponds to 95% filled)."
            ],
            "type": "u64"
          },
          {
            "name": "networkFee",
            "docs": [
              "Tip for maintaining the account."
            ],
            "type": "u64"
          },
          {
            "name": "rolledoverSlot",
            "docs": [
              "The slot when the account was rolled over, a rolled over account should not be written to."
            ],
            "type": "u64"
          },
          {
            "name": "closeThreshold",
            "docs": [
              "If current slot is greater than rolledover_slot + close_threshold and",
              "the account is empty it can be closed."
            ],
            "type": "u64"
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "AddressMerkleTreeConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "height",
            "type": "u32"
          },
          {
            "name": "changelogSize",
            "type": "u64"
          },
          {
            "name": "rootsSize",
            "type": "u64"
          },
          {
            "name": "canopyDepth",
            "type": "u64"
          },
          {
            "name": "addressChangelogSize",
            "type": "u64"
          },
          {
            "name": "networkFee",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "rolloverThreshold",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "closeThreshold",
            "type": {
              "option": "u64"
            }
          }
        ]
      }
    },
    {
      "name": "StateMerkleTreeConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "height",
            "type": "u32"
          },
          {
            "name": "changelogSize",
            "type": "u64"
          },
          {
            "name": "rootsSize",
            "type": "u64"
          },
          {
            "name": "canopyDepth",
            "type": "u64"
          },
          {
            "name": "networkFee",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "rolloverThreshold",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "closeThreshold",
            "type": {
              "option": "u64"
            }
          }
        ]
      }
    },
    {
      "name": "NullifierQueueConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "capacityIndices",
            "type": "u16"
          },
          {
            "name": "capacityValues",
            "type": "u16"
          },
          {
            "name": "sequenceThreshold",
            "type": "u64"
          },
          {
            "name": "networkFee",
            "type": {
              "option": "u64"
            }
          }
        ]
      }
    },
    {
      "name": "AddressQueueConfig",
      "type": {
        "kind": "alias",
        "value": {
          "defined": "NullifierQueueConfig"
        }
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "AddressQueueInsert",
      "msg": "Failed to insert an element into indexing queue"
    },
    {
      "code": 6001,
      "name": "AddressQueueDequeue",
      "msg": "Failed to dequeue an element from indexing queue"
    },
    {
      "code": 6002,
      "name": "AddressMerkleTreeInitialize",
      "msg": "Failed to initialize address Merkle tree"
    },
    {
      "code": 6003,
      "name": "AddressMerkleTreeUpdate",
      "msg": "Failed to update the address Merkle tree"
    },
    {
      "code": 6004,
      "name": "InvalidIndex",
      "msg": "No element found under the given index in the queue"
    },
    {
      "code": 6005,
      "name": "BytesToBigint",
      "msg": "Failed to convert bytes to big integer"
    },
    {
      "code": 6006,
      "name": "IntegerOverflow",
      "msg": "Integer overflow"
    },
    {
      "code": 6007,
      "name": "InvalidAuthority",
      "msg": "InvalidAuthority"
    },
    {
      "code": 6008,
      "name": "InvalidVerifier",
      "msg": "InvalidVerifier"
    },
    {
      "code": 6009,
      "name": "NumberOfLeavesMismatch",
      "msg": "Leaves <> remaining accounts mismatch. The number of remaining accounts must match the number of leaves."
    },
    {
      "code": 6010,
      "name": "InvalidNoopPubkey",
      "msg": "Provided noop program public key is invalid"
    },
    {
      "code": 6011,
      "name": "EventNoChangelogEntry",
      "msg": "Emitting an event requires at least one changelog entry"
    },
    {
      "code": 6012,
      "name": "NumberOfChangeLogIndicesMismatch",
      "msg": "Number of change log indices mismatch"
    },
    {
      "code": 6013,
      "name": "NumberOfIndicesMismatch",
      "msg": "Number of indices mismatch"
    },
    {
      "code": 6014,
      "name": "IndexOutOfBounds",
      "msg": "IndexOutOfBounds"
    },
    {
      "code": 6015,
      "name": "ElementAlreadyExists",
      "msg": "ElementAlreadyExists"
    },
    {
      "code": 6016,
      "name": "HashSetFull",
      "msg": "HashSetFull"
    },
    {
      "code": 6017,
      "name": "NumberOfProofsMismatch",
      "msg": "NumberOfProofsMismatch"
    },
    {
      "code": 6018,
      "name": "InvalidMerkleProof",
      "msg": "InvalidMerkleProof"
    },
    {
      "code": 6019,
      "name": "InvalidNullifierQueue",
      "msg": "InvalidNullifierQueue"
    },
    {
      "code": 6020,
      "name": "InvalidMerkleTree",
      "msg": "InvalidMerkleTree"
    },
    {
      "code": 6021,
      "name": "LeafNotFound",
      "msg": "Could not find the leaf in the queue"
    },
    {
      "code": 6022,
      "name": "RolloverThresholdTooHigh",
      "msg": "RolloverThresholdTooHigh"
    },
    {
      "code": 6023,
      "name": "MerkleTreeAndQueueNotAssociated",
      "msg": "MerkleTreeAndQueueNotAssociated"
    },
    {
      "code": 6024,
      "name": "MerkleTreeAlreadyRolledOver",
      "msg": "MerkleTreeAlreadyRolledOver"
    },
    {
      "code": 6025,
      "name": "NotReadyForRollover",
      "msg": "NotReadyForRollover"
    },
    {
      "code": 6026,
      "name": "RolloverNotConfigured",
      "msg": "RolloverNotConfigured"
    },
    {
      "code": 6027,
      "name": "NotAllLeavesProcessed",
      "msg": "NotAllLeavesProcessed"
    }
  ]
};

export const IDL: AccountCompression = {
  "version": "0.3.5",
  "name": "account_compression",
  "constants": [
    {
      "name": "GROUP_AUTHORITY_SEED",
      "type": "bytes",
      "value": "[103, 114, 111, 117, 112, 95, 97, 117, 116, 104, 111, 114, 105, 116, 121]"
    },
    {
      "name": "STATE_MERKLE_TREE_HEIGHT",
      "type": "u64",
      "value": "26"
    },
    {
      "name": "STATE_MERKLE_TREE_CHANGELOG",
      "type": "u64",
      "value": "1400"
    },
    {
      "name": "STATE_MERKLE_TREE_ROOTS",
      "type": "u64",
      "value": "2400"
    },
    {
      "name": "STATE_MERKLE_TREE_CANOPY_DEPTH",
      "type": "u64",
      "value": "10"
    },
    {
      "name": "STATE_NULLIFIER_QUEUE_INDICES",
      "type": "u16",
      "value": "6857"
    },
    {
      "name": "STATE_NULLIFIER_QUEUE_VALUES",
      "type": "u16",
      "value": "4800"
    },
    {
      "name": "STATE_NULLIFIER_QUEUE_SEQUENCE_THRESHOLD",
      "type": "u64",
      "value": "2400"
    },
    {
      "name": "ADDRESS_MERKLE_TREE_HEIGHT",
      "type": "u64",
      "value": "26"
    },
    {
      "name": "ADDRESS_MERKLE_TREE_CHANGELOG",
      "type": "u64",
      "value": "1400"
    },
    {
      "name": "ADDRESS_MERKLE_TREE_ROOTS",
      "type": "u64",
      "value": "2400"
    },
    {
      "name": "ADDRESS_MERKLE_TREE_CANOPY_DEPTH",
      "type": "u64",
      "value": "10"
    },
    {
      "name": "ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG",
      "type": "u64",
      "value": "256"
    },
    {
      "name": "ADDRESS_QUEUE_INDICES",
      "type": "u16",
      "value": "6857"
    },
    {
      "name": "ADDRESS_QUEUE_VALUES",
      "type": "u16",
      "value": "4800"
    },
    {
      "name": "ADDRESS_QUEUE_SEQUENCE_THRESHOLD",
      "type": "u64",
      "value": "2400"
    },
    {
      "name": "NOOP_PUBKEY",
      "type": {
        "array": [
          "u8",
          32
        ]
      },
      "value": "[11 , 188 , 15 , 192 , 187 , 71 , 202 , 47 , 116 , 196 , 17 , 46 , 148 , 171 , 19 , 207 , 163 , 198 , 52 , 229 , 220 , 23 , 234 , 203 , 3 , 205 , 26 , 35 , 205 , 126 , 120 , 124 ,]"
    },
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN\""
    }
  ],
  "instructions": [
    {
      "name": "initializeAddressMerkleTreeAndQueue",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "queue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u64"
        },
        {
          "name": "owner",
          "type": "publicKey"
        },
        {
          "name": "delegate",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "addressMerkleTreeConfig",
          "type": {
            "defined": "AddressMerkleTreeConfig"
          }
        },
        {
          "name": "addressQueueConfig",
          "type": {
            "defined": "AddressQueueConfig"
          }
        }
      ]
    },
    {
      "name": "insertAddresses",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "addresses",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
          }
        }
      ]
    },
    {
      "name": "updateAddressMerkleTree",
      "docs": [
        "Updates the address Merkle tree with a new address."
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "queue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "logWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "changelogIndex",
          "type": "u16"
        },
        {
          "name": "value",
          "type": "u16"
        },
        {
          "name": "nextIndex",
          "type": "u64"
        },
        {
          "name": "lowAddressIndex",
          "type": "u64"
        },
        {
          "name": "lowAddressValue",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "lowAddressNextIndex",
          "type": "u64"
        },
        {
          "name": "lowAddressNextValue",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "lowAddressProof",
          "type": {
            "array": [
              {
                "array": [
                  "u8",
                  32
                ]
              },
              16
            ]
          }
        }
      ]
    },
    {
      "name": "rolloverAddressMerkleTreeAndQueue",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": false,
          "isSigner": true,
          "docs": [
            "Signer used to pay rollover and protocol fees."
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "newAddressMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "newQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oldAddressMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oldQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "initializeGroupAuthority",
      "docs": [
        "initialize group (a group can be used to give multiple programs acess to the same Merkle trees by registering the programs to the group)"
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "groupAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "seed",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "authority",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "updateGroupAuthority",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "groupAuthority",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "authority",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "registerProgramToGroup",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "groupAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "programId",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "initializeStateMerkleTreeAndNullifierQueue",
      "docs": [
        "Initializes a new Merkle tree from config bytes.",
        "Index is an optional identifier and not checked by the program."
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "nullifierQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u64"
        },
        {
          "name": "owner",
          "type": "publicKey"
        },
        {
          "name": "delegate",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "stateMerkleTreeConfig",
          "type": {
            "defined": "StateMerkleTreeConfig"
          }
        },
        {
          "name": "nullifierQueueConfig",
          "type": {
            "defined": "NullifierQueueConfig"
          }
        }
      ]
    },
    {
      "name": "appendLeavesToMerkleTrees",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Signer used to pay rollover and protocol fees."
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "logWrapper",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "leaves",
          "type": {
            "vec": {
              "defined": "(u8,[u8;32])"
            }
          }
        }
      ]
    },
    {
      "name": "nullifyLeaves",
      "accounts": [
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "logWrapper",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "nullifierQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "changeLogIndices",
          "type": {
            "vec": "u64"
          }
        },
        {
          "name": "leavesQueueIndices",
          "type": {
            "vec": "u16"
          }
        },
        {
          "name": "indices",
          "type": {
            "vec": "u64"
          }
        },
        {
          "name": "proofs",
          "type": {
            "vec": {
              "vec": {
                "array": [
                  "u8",
                  32
                ]
              }
            }
          }
        }
      ]
    },
    {
      "name": "insertIntoNullifierQueues",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "elements",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
          }
        }
      ]
    },
    {
      "name": "rolloverStateMerkleTreeAndNullifierQueue",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": false,
          "isSigner": true,
          "docs": [
            "Signer used to pay rollover and protocol fees."
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registeredProgramPda",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "newStateMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "newNullifierQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oldStateMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oldNullifierQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "groupAuthority",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "publicKey"
          },
          {
            "name": "seed",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "registeredProgram",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pubkey",
            "type": "publicKey"
          }
        ]
      }
    },
    {
      "name": "nullifierQueueAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "metadata",
            "type": {
              "defined": "QueueMetadata"
            }
          }
        ]
      }
    },
    {
      "name": "accessMetadata",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "docs": [
              "Owner of the Merkle tree."
            ],
            "type": "publicKey"
          },
          {
            "name": "delegate",
            "docs": [
              "Delegate of the Merkle tree. This will be used for program owned Merkle trees."
            ],
            "type": "publicKey"
          }
        ]
      }
    },
    {
      "name": "addressQueueAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "metadata",
            "type": {
              "defined": "QueueMetadata"
            }
          }
        ]
      }
    },
    {
      "name": "addressMerkleTreeAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "metadata",
            "type": {
              "defined": "MerkleTreeMetadata"
            }
          },
          {
            "name": "merkleTreeStruct",
            "type": {
              "array": [
                "u8",
                320
              ]
            }
          },
          {
            "name": "merkleTreeFilledSubtrees",
            "type": {
              "array": [
                "u8",
                832
              ]
            }
          },
          {
            "name": "merkleTreeChangelog",
            "type": {
              "array": [
                "u8",
                1220800
              ]
            }
          },
          {
            "name": "merkleTreeRoots",
            "type": {
              "array": [
                "u8",
                76800
              ]
            }
          },
          {
            "name": "merkleTreeCanopy",
            "type": {
              "array": [
                "u8",
                65472
              ]
            }
          },
          {
            "name": "addressChangelog",
            "type": {
              "array": [
                "u8",
                20480
              ]
            }
          }
        ]
      }
    },
    {
      "name": "merkleTreeMetadata",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "accessMetadata",
            "type": {
              "defined": "AccessMetadata"
            }
          },
          {
            "name": "rolloverMetadata",
            "type": {
              "defined": "RolloverMetadata"
            }
          },
          {
            "name": "associatedQueue",
            "type": "publicKey"
          },
          {
            "name": "nextMerkleTree",
            "type": "publicKey"
          }
        ]
      }
    },
    {
      "name": "stateMerkleTreeAccount",
      "docs": [
        "Concurrent state Merkle tree used for public compressed transactions."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "metadata",
            "type": {
              "defined": "MerkleTreeMetadata"
            }
          },
          {
            "name": "stateMerkleTreeStruct",
            "docs": [
              "Merkle tree for the transaction state."
            ],
            "type": {
              "array": [
                "u8",
                272
              ]
            }
          },
          {
            "name": "stateMerkleTreeFilledSubtrees",
            "type": {
              "array": [
                "u8",
                832
              ]
            }
          },
          {
            "name": "stateMerkleTreeChangelog",
            "type": {
              "array": [
                "u8",
                1220800
              ]
            }
          },
          {
            "name": "stateMerkleTreeRoots",
            "type": {
              "array": [
                "u8",
                76800
              ]
            }
          },
          {
            "name": "stateMerkleTreeCanopy",
            "type": {
              "array": [
                "u8",
                65472
              ]
            }
          }
        ]
      }
    },
    {
      "name": "queueMetadata",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "accessMetadata",
            "type": {
              "defined": "AccessMetadata"
            }
          },
          {
            "name": "rolloverMetadata",
            "type": {
              "defined": "RolloverMetadata"
            }
          },
          {
            "name": "associatedMerkleTree",
            "type": "publicKey"
          },
          {
            "name": "nextQueue",
            "type": "publicKey"
          }
        ]
      }
    },
    {
      "name": "rolloverMetadata",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "index",
            "docs": [
              "Unique index."
            ],
            "type": "u64"
          },
          {
            "name": "rolloverFee",
            "docs": [
              "This fee is used for rent for the next account.",
              "It accumulates in the account so that once the corresponding Merkle tree account is full it can be rolled over"
            ],
            "type": "u64"
          },
          {
            "name": "rolloverThreshold",
            "docs": [
              "The threshold in percentage points when the account should be rolled over (95 corresponds to 95% filled)."
            ],
            "type": "u64"
          },
          {
            "name": "networkFee",
            "docs": [
              "Tip for maintaining the account."
            ],
            "type": "u64"
          },
          {
            "name": "rolledoverSlot",
            "docs": [
              "The slot when the account was rolled over, a rolled over account should not be written to."
            ],
            "type": "u64"
          },
          {
            "name": "closeThreshold",
            "docs": [
              "If current slot is greater than rolledover_slot + close_threshold and",
              "the account is empty it can be closed."
            ],
            "type": "u64"
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "AddressMerkleTreeConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "height",
            "type": "u32"
          },
          {
            "name": "changelogSize",
            "type": "u64"
          },
          {
            "name": "rootsSize",
            "type": "u64"
          },
          {
            "name": "canopyDepth",
            "type": "u64"
          },
          {
            "name": "addressChangelogSize",
            "type": "u64"
          },
          {
            "name": "networkFee",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "rolloverThreshold",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "closeThreshold",
            "type": {
              "option": "u64"
            }
          }
        ]
      }
    },
    {
      "name": "StateMerkleTreeConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "height",
            "type": "u32"
          },
          {
            "name": "changelogSize",
            "type": "u64"
          },
          {
            "name": "rootsSize",
            "type": "u64"
          },
          {
            "name": "canopyDepth",
            "type": "u64"
          },
          {
            "name": "networkFee",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "rolloverThreshold",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "closeThreshold",
            "type": {
              "option": "u64"
            }
          }
        ]
      }
    },
    {
      "name": "NullifierQueueConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "capacityIndices",
            "type": "u16"
          },
          {
            "name": "capacityValues",
            "type": "u16"
          },
          {
            "name": "sequenceThreshold",
            "type": "u64"
          },
          {
            "name": "networkFee",
            "type": {
              "option": "u64"
            }
          }
        ]
      }
    },
    {
      "name": "AddressQueueConfig",
      "type": {
        "kind": "alias",
        "value": {
          "defined": "NullifierQueueConfig"
        }
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "AddressQueueInsert",
      "msg": "Failed to insert an element into indexing queue"
    },
    {
      "code": 6001,
      "name": "AddressQueueDequeue",
      "msg": "Failed to dequeue an element from indexing queue"
    },
    {
      "code": 6002,
      "name": "AddressMerkleTreeInitialize",
      "msg": "Failed to initialize address Merkle tree"
    },
    {
      "code": 6003,
      "name": "AddressMerkleTreeUpdate",
      "msg": "Failed to update the address Merkle tree"
    },
    {
      "code": 6004,
      "name": "InvalidIndex",
      "msg": "No element found under the given index in the queue"
    },
    {
      "code": 6005,
      "name": "BytesToBigint",
      "msg": "Failed to convert bytes to big integer"
    },
    {
      "code": 6006,
      "name": "IntegerOverflow",
      "msg": "Integer overflow"
    },
    {
      "code": 6007,
      "name": "InvalidAuthority",
      "msg": "InvalidAuthority"
    },
    {
      "code": 6008,
      "name": "InvalidVerifier",
      "msg": "InvalidVerifier"
    },
    {
      "code": 6009,
      "name": "NumberOfLeavesMismatch",
      "msg": "Leaves <> remaining accounts mismatch. The number of remaining accounts must match the number of leaves."
    },
    {
      "code": 6010,
      "name": "InvalidNoopPubkey",
      "msg": "Provided noop program public key is invalid"
    },
    {
      "code": 6011,
      "name": "EventNoChangelogEntry",
      "msg": "Emitting an event requires at least one changelog entry"
    },
    {
      "code": 6012,
      "name": "NumberOfChangeLogIndicesMismatch",
      "msg": "Number of change log indices mismatch"
    },
    {
      "code": 6013,
      "name": "NumberOfIndicesMismatch",
      "msg": "Number of indices mismatch"
    },
    {
      "code": 6014,
      "name": "IndexOutOfBounds",
      "msg": "IndexOutOfBounds"
    },
    {
      "code": 6015,
      "name": "ElementAlreadyExists",
      "msg": "ElementAlreadyExists"
    },
    {
      "code": 6016,
      "name": "HashSetFull",
      "msg": "HashSetFull"
    },
    {
      "code": 6017,
      "name": "NumberOfProofsMismatch",
      "msg": "NumberOfProofsMismatch"
    },
    {
      "code": 6018,
      "name": "InvalidMerkleProof",
      "msg": "InvalidMerkleProof"
    },
    {
      "code": 6019,
      "name": "InvalidNullifierQueue",
      "msg": "InvalidNullifierQueue"
    },
    {
      "code": 6020,
      "name": "InvalidMerkleTree",
      "msg": "InvalidMerkleTree"
    },
    {
      "code": 6021,
      "name": "LeafNotFound",
      "msg": "Could not find the leaf in the queue"
    },
    {
      "code": 6022,
      "name": "RolloverThresholdTooHigh",
      "msg": "RolloverThresholdTooHigh"
    },
    {
      "code": 6023,
      "name": "MerkleTreeAndQueueNotAssociated",
      "msg": "MerkleTreeAndQueueNotAssociated"
    },
    {
      "code": 6024,
      "name": "MerkleTreeAlreadyRolledOver",
      "msg": "MerkleTreeAlreadyRolledOver"
    },
    {
      "code": 6025,
      "name": "NotReadyForRollover",
      "msg": "NotReadyForRollover"
    },
    {
      "code": 6026,
      "name": "RolloverNotConfigured",
      "msg": "RolloverNotConfigured"
    },
    {
      "code": 6027,
      "name": "NotAllLeavesProcessed",
      "msg": "NotAllLeavesProcessed"
    }
  ]
};
