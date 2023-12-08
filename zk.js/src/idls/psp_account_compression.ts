export type PspAccountCompression = {
  "version": "0.3.1",
  "name": "psp_account_compression",
  "constants": [
    {
      "name": "ENCRYPTED_UTXOS_LENGTH",
      "type": {
        "defined": "usize"
      },
      "value": "174"
    },
    {
      "name": "MERKLE_TREE_HEIGHT",
      "type": {
        "defined": "usize"
      },
      "value": "22"
    },
    {
      "name": "MERKLE_TREE_CHANGELOG",
      "type": {
        "defined": "usize"
      },
      "value": "0"
    },
    {
      "name": "MERKLE_TREE_ROOTS",
      "type": {
        "defined": "usize"
      },
      "value": "2800"
    },
    {
      "name": "INITIAL_MERKLE_TREE_AUTHORITY",
      "type": {
        "array": [
          "u8",
          32
        ]
      },
      "value": "[2 , 99 , 226 , 251 , 88 , 66 , 92 , 33 , 25 , 216 , 211 , 185 , 112 , 203 , 212 , 238 , 105 , 144 , 72 , 121 , 176 , 253 , 106 , 168 , 115 , 158 , 154 , 188 , 62 , 255 , 166 , 81]"
    },
    {
      "name": "IX_ORDER",
      "type": {
        "array": [
          "u8",
          57
        ]
      },
      "value": "[34 , 14 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 241]"
    },
    {
      "name": "MERKLE_TREE_UPDATE_START",
      "type": "u8",
      "value": "14"
    },
    {
      "name": "LOCK_START",
      "type": "u8",
      "value": "34"
    },
    {
      "name": "HASH_0",
      "type": "u8",
      "value": "0"
    },
    {
      "name": "HASH_1",
      "type": "u8",
      "value": "1"
    },
    {
      "name": "HASH_2",
      "type": "u8",
      "value": "2"
    },
    {
      "name": "ROOT_INSERT",
      "type": "u8",
      "value": "241"
    },
    {
      "name": "AUTHORITY_SEED",
      "type": "bytes",
      "value": "[65, 85, 84, 72, 79, 82, 73, 84, 89, 95, 83, 69, 69, 68]"
    },
    {
      "name": "MERKLE_TREE_AUTHORITY_SEED",
      "type": "bytes",
      "value": "[77, 69, 82, 75, 76, 69, 95, 84, 82, 69, 69, 95, 65, 85, 84, 72, 79, 82, 73, 84, 89]"
    },
    {
      "name": "TREE_ROOT_SEED",
      "type": "bytes",
      "value": "[84, 82, 69, 69, 95, 82, 79, 79, 84, 95, 83, 69, 69, 68]"
    },
    {
      "name": "STORAGE_SEED",
      "type": "bytes",
      "value": "[115, 116, 111, 114, 97, 103, 101]"
    },
    {
      "name": "LEAVES_SEED",
      "type": "bytes",
      "value": "[108, 101, 97, 118, 101, 115]"
    },
    {
      "name": "NULLIFIER_SEED",
      "type": "bytes",
      "value": "[110, 102]"
    },
    {
      "name": "POOL_TYPE_SEED",
      "type": "bytes",
      "value": "[112, 111, 111, 108, 116, 121, 112, 101]"
    },
    {
      "name": "POOL_CONFIG_SEED",
      "type": "bytes",
      "value": "[112, 111, 111, 108, 45, 99, 111, 110, 102, 105, 103]"
    },
    {
      "name": "POOL_SEED",
      "type": "bytes",
      "value": "[112, 111, 111, 108]"
    },
    {
      "name": "TOKEN_AUTHORITY_SEED",
      "type": "bytes",
      "value": "[115, 112, 108]"
    }
  ],
  "instructions": [
    {
      "name": "initializeConcurrentMerkleTree",
      "docs": [
        "Initializes a new Merkle tree from config bytes.",
        "Can only be called from the merkle_tree_authority."
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
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "insertLeavesParallel",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "leaves",
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
    }
  ],
  "accounts": [
    {
      "name": "indexedArrayAccount",
      "type": {
        "kind": "struct",
        "fields": [
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
            "type": "publicKey"
          },
          {
            "name": "array",
            "type": "publicKey"
          },
          {
            "name": "indexedArray",
            "type": {
              "array": [
                "u8",
                112008
              ]
            }
          }
        ]
      }
    },
    {
      "name": "concurrentMerkleTreeAccount",
      "docs": [
        "Concurrent state Merkle tree used for public compressed transactions."
      ],
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
            "name": "nextMerkleTree",
            "docs": [
              "Public key of the next Merkle tree."
            ],
            "type": "publicKey"
          },
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
              "Delegate of the Merkle tree."
            ],
            "type": "publicKey"
          },
          {
            "name": "stateMerkleTree",
            "docs": [
              "Merkle tree for the transaction state."
            ],
            "type": {
              "array": [
                "u8",
                90360
              ]
            }
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "MtTmpPdaInitFailed",
      "msg": "Merkle tree tmp account init failed wrong pda."
    },
    {
      "code": 6001,
      "name": "MerkleTreeInitFailed",
      "msg": "Merkle tree tmp account init failed."
    },
    {
      "code": 6002,
      "name": "ContractStillLocked",
      "msg": "Contract is still locked."
    },
    {
      "code": 6003,
      "name": "InvalidMerkleTree",
      "msg": "InvalidMerkleTree."
    },
    {
      "code": 6004,
      "name": "InvalidMerkleTreeOwner",
      "msg": "InvalidMerkleTreeOwner."
    },
    {
      "code": 6005,
      "name": "PubkeyCheckFailed",
      "msg": "PubkeyCheckFailed"
    },
    {
      "code": 6006,
      "name": "CloseAccountFailed",
      "msg": "CloseAccountFailed"
    },
    {
      "code": 6007,
      "name": "DecompressFailed",
      "msg": "DecompressFailed"
    },
    {
      "code": 6008,
      "name": "MerkleTreeUpdateNotInRootInsert",
      "msg": "MerkleTreeUpdateNotInRootInsert"
    },
    {
      "code": 6009,
      "name": "MerkleTreeUpdateNotInRootInsertState",
      "msg": "MerkleTreeUpdateNotInRootInsert"
    },
    {
      "code": 6010,
      "name": "InvalidNumberOfLeaves",
      "msg": "InvalidNumberOfLeaves"
    },
    {
      "code": 6011,
      "name": "LeafAlreadyInserted",
      "msg": "LeafAlreadyInserted"
    },
    {
      "code": 6012,
      "name": "WrongLeavesLastTx",
      "msg": "WrongLeavesLastTx"
    },
    {
      "code": 6013,
      "name": "FirstLeavesPdaIncorrectIndex",
      "msg": "FirstLeavesPdaIncorrectIndex"
    },
    {
      "code": 6014,
      "name": "NullifierAlreadyExists",
      "msg": "NullifierAlreadyExists"
    },
    {
      "code": 6015,
      "name": "LeavesOfWrongTree",
      "msg": "LeavesOfWrongTree"
    },
    {
      "code": 6016,
      "name": "InvalidAuthority",
      "msg": "InvalidAuthority"
    },
    {
      "code": 6017,
      "name": "InvalidVerifier",
      "msg": "InvalidVerifier"
    },
    {
      "code": 6018,
      "name": "PubkeyTryFromFailed",
      "msg": "PubkeyTryFromFailed"
    },
    {
      "code": 6019,
      "name": "ExpectedOldMerkleTrees",
      "msg": "Expected old Merkle trees as remaining account."
    },
    {
      "code": 6020,
      "name": "InvalidOldMerkleTree",
      "msg": "Invalid old Merkle tree account."
    },
    {
      "code": 6021,
      "name": "NotNewestOldMerkleTree",
      "msg": "Provided old Merkle tree is not the newest one."
    },
    {
      "code": 6022,
      "name": "ExpectedTwoLeavesPda",
      "msg": "Expected two leaves PDA as a remaining account."
    },
    {
      "code": 6023,
      "name": "InvalidTwoLeavesPda",
      "msg": "Invalid two leaves PDA."
    },
    {
      "code": 6024,
      "name": "OddNumberOfLeaves",
      "msg": "Odd number of leaves."
    }
  ]
};

export const IDL: PspAccountCompression = {
  "version": "0.3.1",
  "name": "psp_account_compression",
  "constants": [
    {
      "name": "ENCRYPTED_UTXOS_LENGTH",
      "type": {
        "defined": "usize"
      },
      "value": "174"
    },
    {
      "name": "MERKLE_TREE_HEIGHT",
      "type": {
        "defined": "usize"
      },
      "value": "22"
    },
    {
      "name": "MERKLE_TREE_CHANGELOG",
      "type": {
        "defined": "usize"
      },
      "value": "0"
    },
    {
      "name": "MERKLE_TREE_ROOTS",
      "type": {
        "defined": "usize"
      },
      "value": "2800"
    },
    {
      "name": "INITIAL_MERKLE_TREE_AUTHORITY",
      "type": {
        "array": [
          "u8",
          32
        ]
      },
      "value": "[2 , 99 , 226 , 251 , 88 , 66 , 92 , 33 , 25 , 216 , 211 , 185 , 112 , 203 , 212 , 238 , 105 , 144 , 72 , 121 , 176 , 253 , 106 , 168 , 115 , 158 , 154 , 188 , 62 , 255 , 166 , 81]"
    },
    {
      "name": "IX_ORDER",
      "type": {
        "array": [
          "u8",
          57
        ]
      },
      "value": "[34 , 14 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 0 , 1 , 2 , 241]"
    },
    {
      "name": "MERKLE_TREE_UPDATE_START",
      "type": "u8",
      "value": "14"
    },
    {
      "name": "LOCK_START",
      "type": "u8",
      "value": "34"
    },
    {
      "name": "HASH_0",
      "type": "u8",
      "value": "0"
    },
    {
      "name": "HASH_1",
      "type": "u8",
      "value": "1"
    },
    {
      "name": "HASH_2",
      "type": "u8",
      "value": "2"
    },
    {
      "name": "ROOT_INSERT",
      "type": "u8",
      "value": "241"
    },
    {
      "name": "AUTHORITY_SEED",
      "type": "bytes",
      "value": "[65, 85, 84, 72, 79, 82, 73, 84, 89, 95, 83, 69, 69, 68]"
    },
    {
      "name": "MERKLE_TREE_AUTHORITY_SEED",
      "type": "bytes",
      "value": "[77, 69, 82, 75, 76, 69, 95, 84, 82, 69, 69, 95, 65, 85, 84, 72, 79, 82, 73, 84, 89]"
    },
    {
      "name": "TREE_ROOT_SEED",
      "type": "bytes",
      "value": "[84, 82, 69, 69, 95, 82, 79, 79, 84, 95, 83, 69, 69, 68]"
    },
    {
      "name": "STORAGE_SEED",
      "type": "bytes",
      "value": "[115, 116, 111, 114, 97, 103, 101]"
    },
    {
      "name": "LEAVES_SEED",
      "type": "bytes",
      "value": "[108, 101, 97, 118, 101, 115]"
    },
    {
      "name": "NULLIFIER_SEED",
      "type": "bytes",
      "value": "[110, 102]"
    },
    {
      "name": "POOL_TYPE_SEED",
      "type": "bytes",
      "value": "[112, 111, 111, 108, 116, 121, 112, 101]"
    },
    {
      "name": "POOL_CONFIG_SEED",
      "type": "bytes",
      "value": "[112, 111, 111, 108, 45, 99, 111, 110, 102, 105, 103]"
    },
    {
      "name": "POOL_SEED",
      "type": "bytes",
      "value": "[112, 111, 111, 108]"
    },
    {
      "name": "TOKEN_AUTHORITY_SEED",
      "type": "bytes",
      "value": "[115, 112, 108]"
    }
  ],
  "instructions": [
    {
      "name": "initializeConcurrentMerkleTree",
      "docs": [
        "Initializes a new Merkle tree from config bytes.",
        "Can only be called from the merkle_tree_authority."
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
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "insertLeavesParallel",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "leaves",
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
    }
  ],
  "accounts": [
    {
      "name": "indexedArrayAccount",
      "type": {
        "kind": "struct",
        "fields": [
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
            "type": "publicKey"
          },
          {
            "name": "array",
            "type": "publicKey"
          },
          {
            "name": "indexedArray",
            "type": {
              "array": [
                "u8",
                112008
              ]
            }
          }
        ]
      }
    },
    {
      "name": "concurrentMerkleTreeAccount",
      "docs": [
        "Concurrent state Merkle tree used for public compressed transactions."
      ],
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
            "name": "nextMerkleTree",
            "docs": [
              "Public key of the next Merkle tree."
            ],
            "type": "publicKey"
          },
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
              "Delegate of the Merkle tree."
            ],
            "type": "publicKey"
          },
          {
            "name": "stateMerkleTree",
            "docs": [
              "Merkle tree for the transaction state."
            ],
            "type": {
              "array": [
                "u8",
                90360
              ]
            }
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "MtTmpPdaInitFailed",
      "msg": "Merkle tree tmp account init failed wrong pda."
    },
    {
      "code": 6001,
      "name": "MerkleTreeInitFailed",
      "msg": "Merkle tree tmp account init failed."
    },
    {
      "code": 6002,
      "name": "ContractStillLocked",
      "msg": "Contract is still locked."
    },
    {
      "code": 6003,
      "name": "InvalidMerkleTree",
      "msg": "InvalidMerkleTree."
    },
    {
      "code": 6004,
      "name": "InvalidMerkleTreeOwner",
      "msg": "InvalidMerkleTreeOwner."
    },
    {
      "code": 6005,
      "name": "PubkeyCheckFailed",
      "msg": "PubkeyCheckFailed"
    },
    {
      "code": 6006,
      "name": "CloseAccountFailed",
      "msg": "CloseAccountFailed"
    },
    {
      "code": 6007,
      "name": "DecompressFailed",
      "msg": "DecompressFailed"
    },
    {
      "code": 6008,
      "name": "MerkleTreeUpdateNotInRootInsert",
      "msg": "MerkleTreeUpdateNotInRootInsert"
    },
    {
      "code": 6009,
      "name": "MerkleTreeUpdateNotInRootInsertState",
      "msg": "MerkleTreeUpdateNotInRootInsert"
    },
    {
      "code": 6010,
      "name": "InvalidNumberOfLeaves",
      "msg": "InvalidNumberOfLeaves"
    },
    {
      "code": 6011,
      "name": "LeafAlreadyInserted",
      "msg": "LeafAlreadyInserted"
    },
    {
      "code": 6012,
      "name": "WrongLeavesLastTx",
      "msg": "WrongLeavesLastTx"
    },
    {
      "code": 6013,
      "name": "FirstLeavesPdaIncorrectIndex",
      "msg": "FirstLeavesPdaIncorrectIndex"
    },
    {
      "code": 6014,
      "name": "NullifierAlreadyExists",
      "msg": "NullifierAlreadyExists"
    },
    {
      "code": 6015,
      "name": "LeavesOfWrongTree",
      "msg": "LeavesOfWrongTree"
    },
    {
      "code": 6016,
      "name": "InvalidAuthority",
      "msg": "InvalidAuthority"
    },
    {
      "code": 6017,
      "name": "InvalidVerifier",
      "msg": "InvalidVerifier"
    },
    {
      "code": 6018,
      "name": "PubkeyTryFromFailed",
      "msg": "PubkeyTryFromFailed"
    },
    {
      "code": 6019,
      "name": "ExpectedOldMerkleTrees",
      "msg": "Expected old Merkle trees as remaining account."
    },
    {
      "code": 6020,
      "name": "InvalidOldMerkleTree",
      "msg": "Invalid old Merkle tree account."
    },
    {
      "code": 6021,
      "name": "NotNewestOldMerkleTree",
      "msg": "Provided old Merkle tree is not the newest one."
    },
    {
      "code": 6022,
      "name": "ExpectedTwoLeavesPda",
      "msg": "Expected two leaves PDA as a remaining account."
    },
    {
      "code": 6023,
      "name": "InvalidTwoLeavesPda",
      "msg": "Invalid two leaves PDA."
    },
    {
      "code": 6024,
      "name": "OddNumberOfLeaves",
      "msg": "Odd number of leaves."
    }
  ]
};
