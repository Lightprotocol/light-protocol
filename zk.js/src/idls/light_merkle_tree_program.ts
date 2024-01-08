export type LightMerkleTreeProgram = {
  "version": "0.3.1",
  "name": "light_merkle_tree_program",
  "constants": [
    {
      "name": "ENCRYPTED_UTXOS_LENGTH",
      "type": {
        "defined": "usize"
      },
      "value": "174"
    },
    {
      "name": "MERKLE_TREE_HISTORY_SIZE",
      "type": "u64",
      "value": "256"
    },
    {
      "name": "MERKLE_TREE_HEIGHT",
      "type": {
        "defined": "usize"
      },
      "value": "18"
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
      "name": "ZERO_BYTES_MERKLE_TREE_18",
      "type": {
        "array": [
          {
            "array": [
              "u8",
              32
            ]
          },
          19
        ]
      },
      "value": "[[40 , 66 , 58 , 227 , 48 , 224 , 249 , 227 , 188 , 18 , 133 , 168 , 156 , 214 , 220 , 144 , 244 , 144 , 67 , 82 , 76 , 6 , 135 , 78 , 64 , 186 , 52 , 113 , 234 , 47 , 27 , 32] , [227 , 42 , 164 , 149 , 188 , 70 , 170 , 8 , 197 , 44 , 134 , 162 , 211 , 186 , 50 , 238 , 97 , 71 , 25 , 130 , 77 , 70 , 37 , 128 , 172 , 154 , 54 , 111 , 93 , 193 , 105 , 27] , [25 , 241 , 255 , 33 , 65 , 214 , 48 , 229 , 38 , 116 , 134 , 103 , 44 , 146 , 163 , 214 , 31 , 238 , 148 , 206 , 34 , 137 , 144 , 221 , 184 , 11 , 5 , 213 , 10 , 188 , 143 , 18] , [211 , 61 , 251 , 33 , 128 , 34 , 4 , 100 , 229 , 47 , 99 , 121 , 109 , 204 , 224 , 90 , 200 , 149 , 219 , 20 , 48 , 206 , 210 , 177 , 161 , 66 , 44 , 10 , 169 , 56 , 248 , 8] , [200 , 15 , 65 , 80 , 151 , 74 , 72 , 69 , 229 , 131 , 25 , 215 , 86 , 36 , 195 , 74 , 67 , 59 , 117 , 179 , 51 , 60 , 181 , 13 , 242 , 192 , 228 , 228 , 189 , 238 , 70 , 8] , [171 , 62 , 122 , 81 , 181 , 197 , 22 , 238 , 224 , 40 , 154 , 231 , 127 , 202 , 201 , 169 , 196 , 109 , 244 , 175 , 117 , 101 , 23 , 67 , 103 , 57 , 127 , 200 , 37 , 43 , 111 , 7] , [59 , 78 , 126 , 104 , 199 , 143 , 213 , 10 , 2 , 158 , 64 , 78 , 153 , 25 , 107 , 190 , 32 , 122 , 123 , 211 , 116 , 179 , 175 , 172 , 70 , 54 , 175 , 59 , 201 , 120 , 64 , 44] , [110 , 91 , 92 , 81 , 205 , 89 , 122 , 223 , 55 , 163 , 42 , 227 , 109 , 54 , 38 , 22 , 110 , 217 , 29 , 148 , 107 , 99 , 128 , 106 , 146 , 47 , 239 , 41 , 55 , 157 , 155 , 22] , [18 , 231 , 42 , 5 , 245 , 159 , 211 , 227 , 239 , 89 , 35 , 142 , 223 , 69 , 166 , 224 , 14 , 114 , 128 , 14 , 123 , 123 , 215 , 2 , 241 , 185 , 191 , 60 , 252 , 61 , 146 , 12] , [231 , 0 , 84 , 227 , 127 , 64 , 158 , 7 , 171 , 179 , 137 , 231 , 92 , 87 , 25 , 221 , 156 , 229 , 53 , 208 , 194 , 201 , 12 , 165 , 105 , 150 , 41 , 142 , 29 , 205 , 136 , 29] , [195 , 2 , 103 , 231 , 62 , 207 , 214 , 105 , 214 , 210 , 108 , 23 , 28 , 151 , 77 , 100 , 78 , 194 , 210 , 29 , 227 , 14 , 17 , 242 , 211 , 50 , 33 , 194 , 106 , 18 , 246 , 45] , [131 , 178 , 24 , 157 , 251 , 247 , 103 , 69 , 101 , 229 , 194 , 14 , 167 , 57 , 158 , 128 , 212 , 19 , 140 , 234 , 69 , 37 , 10 , 156 , 249 , 96 , 152 , 52 , 97 , 96 , 119 , 41] , [30 , 223 , 20 , 181 , 108 , 110 , 112 , 102 , 234 , 54 , 99 , 29 , 213 , 3 , 55 , 225 , 125 , 185 , 223 , 234 , 188 , 108 , 83 , 89 , 27 , 3 , 100 , 6 , 65 , 107 , 3 , 24] , [167 , 32 , 85 , 233 , 205 , 253 , 154 , 214 , 236 , 82 , 147 , 75 , 252 , 144 , 109 , 73 , 63 , 167 , 77 , 233 , 12 , 201 , 150 , 242 , 103 , 15 , 158 , 83 , 137 , 24 , 170 , 16] , [45 , 98 , 238 , 69 , 136 , 141 , 101 , 226 , 94 , 209 , 58 , 215 , 212 , 14 , 210 , 135 , 110 , 96 , 52 , 16 , 101 , 177 , 121 , 109 , 134 , 81 , 189 , 146 , 113 , 243 , 97 , 42] , [71 , 51 , 251 , 48 , 95 , 193 , 94 , 26 , 180 , 17 , 124 , 203 , 48 , 98 , 55 , 17 , 60 , 104 , 186 , 175 , 213 , 189 , 7 , 239 , 92 , 175 , 16 , 5 , 220 , 168 , 70 , 21] , [35 , 92 , 72 , 197 , 23 , 142 , 16 , 200 , 136 , 38 , 44 , 255 , 162 , 115 , 11 , 1 , 248 , 182 , 236 , 78 , 90 , 24 , 128 , 245 , 168 , 17 , 130 , 2 , 73 , 51 , 196 , 6] , [89 , 178 , 154 , 246 , 236 , 130 , 30 , 100 , 27 , 230 , 24 , 196 , 8 , 172 , 176 , 196 , 197 , 13 , 157 , 194 , 169 , 106 , 207 , 70 , 66 , 117 , 69 , 53 , 56 , 154 , 78 , 0] , [231 , 174 , 226 , 37 , 211 , 160 , 187 , 178 , 149 , 82 , 17 , 60 , 110 , 116 , 28 , 61 , 58 , 145 , 58 , 71 , 25 , 42 , 67 , 46 , 189 , 214 , 248 , 234 , 182 , 251 , 238 , 34]]"
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
    },
    {
      "name": "EVENT_MERKLE_TREE_SEED",
      "type": "bytes",
      "value": "[101, 118, 101, 110, 116, 95, 109, 101, 114, 107, 108, 101, 95, 116, 114, 101, 101]"
    },
    {
      "name": "TRANSACTION_MERKLE_TREE_SEED",
      "type": "bytes",
      "value": "[116, 114, 97, 110, 115, 97, 99, 116, 105, 111, 110, 95, 109, 101, 114, 107, 108, 101, 95, 116, 114, 101, 101]"
    }
  ],
  "instructions": [
    {
      "name": "initializeNewMerkleTrees",
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
          "name": "newTransactionMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "newEventMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "lockDuration",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initializeMerkleTreeAuthority",
      "docs": [
        "Initializes a new merkle tree authority which can register new verifiers and configure",
        "permissions to create new pools."
      ],
      "accounts": [
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "transactionMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "eventMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "updateMerkleTreeAuthority",
      "docs": [
        "Updates the merkle tree authority to a new authority."
      ],
      "accounts": [
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "newAuthority",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "enablePermissionlessSplTokens",
      "docs": [
        "Enables anyone to create token pools."
      ],
      "accounts": [
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "enablePermissionless",
          "type": "bool"
        }
      ]
    },
    {
      "name": "registerVerifier",
      "docs": [
        "Registers a new verifier which can unshield tokens, insert new nullifiers, add new leaves.",
        "These functions can only be invoked from registered verifiers."
      ],
      "accounts": [
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "verifierPubkey",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "registerPoolType",
      "docs": [
        "Registers a new pooltype."
      ],
      "accounts": [
        {
          "name": "registeredPoolTypePda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "poolType",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    },
    {
      "name": "registerSplPool",
      "docs": [
        "Creates a new spl token pool which can be used by any registered verifier."
      ],
      "accounts": [
        {
          "name": "registeredAssetPoolPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "merkleTreePdaToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "mint",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredPoolTypePda",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Just needs to exist and be derived correctly."
          ]
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "registerSolPool",
      "docs": [
        "Creates a new sol pool which can be used by any registered verifier."
      ],
      "accounts": [
        {
          "name": "registeredAssetPoolPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredPoolTypePda",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "insertTwoLeaves",
      "docs": [
        "Creates and initializes a pda which stores two merkle tree leaves and encrypted Utxos.",
        "The inserted leaves are not part of the Merkle tree yet and marked accordingly.",
        "The Merkle tree has to be updated after.",
        "Can only be called from a registered verifier program."
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "transactionMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": false,
          "isSigner": false
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
    },
    {
      "name": "insertTwoLeavesEvent",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "eventMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredVerifier",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "leafLeft",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "leafRight",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    },
    {
      "name": "unshieldSol",
      "docs": [
        "Unshields sol from a liquidity pool.",
        "An arbitrary number of recipients can be passed in with remaining accounts.",
        "Can only be called from a registered verifier program."
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTreeToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipient",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "unshieldSpl",
      "docs": [
        "Unshields spl tokens from a liquidity pool.",
        "An arbitrary number of recipients can be passed in with remaining accounts.",
        "Can only be called from a registered verifier program."
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTreeToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipient",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initializeNullifiers",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "nullifiers",
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
      "name": "registeredAssetPool",
      "docs": [
        "Nullfier pdas are derived from the nullifier",
        "existence of a nullifier is the check to prevent double spends."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "assetPoolPubkey",
            "type": "publicKey"
          },
          {
            "name": "poolType",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "index",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "registeredPoolType",
      "docs": [
        "Pool type"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "poolType",
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
      "name": "eventMerkleTree",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "merkleTreeNr",
            "type": "u64"
          },
          {
            "name": "newest",
            "type": "u8"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "merkleTree",
            "type": {
              "defined": "MerkleTree"
            }
          }
        ]
      }
    },
    {
      "name": "merkleTreePdaToken",
      "type": {
        "kind": "struct",
        "fields": []
      }
    },
    {
      "name": "preInsertedLeavesIndex",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nextIndex",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "merkleTreeAuthority",
      "docs": [
        "Configures the authority of the merkle tree which can:",
        "- register new verifiers",
        "- register new asset pools",
        "- register new asset pool types",
        "- set permissions for new asset pool creation",
        "- keeps current highest index for assets and merkle trees to enable lookups of these"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pubkey",
            "type": "publicKey"
          },
          {
            "name": "transactionMerkleTreeIndex",
            "type": "u64"
          },
          {
            "name": "eventMerkleTreeIndex",
            "type": "u64"
          },
          {
            "name": "registeredAssetIndex",
            "type": "u64"
          },
          {
            "name": "enablePermissionlessSplTokens",
            "type": "bool"
          },
          {
            "name": "enablePermissionlessMerkleTreeRegistration",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "merkleTreeUpdateState",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nodeLeft",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "nodeRight",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "leafLeft",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "leafRight",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "rpc",
            "type": "publicKey"
          },
          {
            "name": "merkleTreePdaPubkey",
            "type": "publicKey"
          },
          {
            "name": "state",
            "type": {
              "array": [
                "u8",
                96
              ]
            }
          },
          {
            "name": "currentRound",
            "type": "u64"
          },
          {
            "name": "currentRoundIndex",
            "type": "u64"
          },
          {
            "name": "currentInstructionIndex",
            "type": "u64"
          },
          {
            "name": "currentIndex",
            "type": "u64"
          },
          {
            "name": "currentLevel",
            "type": "u64"
          },
          {
            "name": "currentLevelHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "tmpLeavesIndex",
            "type": "u64"
          },
          {
            "name": "filledSubtrees",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                32
              ]
            }
          },
          {
            "name": "leaves",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        32
                      ]
                    },
                    2
                  ]
                },
                16
              ]
            }
          },
          {
            "name": "numberOfLeaves",
            "type": "u8"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "insertLeavesIndex",
            "type": "u8"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          }
        ]
      }
    },
    {
      "name": "registeredVerifier",
      "docs": [
        ""
      ],
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
      "name": "transactionMerkleTree",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "merkleTreeNr",
            "type": "u64"
          },
          {
            "name": "newest",
            "type": "u64"
          },
          {
            "name": "merkleTree",
            "type": {
              "defined": "MerkleTree"
            }
          }
        ]
      }
    },
    {
      "name": "twoLeavesBytesPda",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nodeLeft",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "nodeRight",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "merkleTreePubkey",
            "type": "publicKey"
          },
          {
            "name": "leftLeafIndex",
            "type": "u64"
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "MerkleTree",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "height",
            "docs": [
              "Height of the Merkle tree."
            ],
            "type": "u64"
          },
          {
            "name": "filledSubtrees",
            "docs": [
              "Subtree hashes."
            ],
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                32
              ]
            }
          },
          {
            "name": "roots",
            "docs": [
              "Full history of roots of the Merkle tree (the last one is the current",
              "one)."
            ],
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                256
              ]
            }
          },
          {
            "name": "nextIndex",
            "docs": [
              "Next index to insert a leaf."
            ],
            "type": "u64"
          },
          {
            "name": "currentRootIndex",
            "docs": [
              "Current index of the root."
            ],
            "type": "u64"
          },
          {
            "name": "hashFunction",
            "docs": [
              "Hash implementation used on the Merkle tree."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "HashFunction",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Sha256"
          },
          {
            "name": "Poseidon"
          }
        ]
      }
    },
    {
      "name": "MerkleTreeError",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "HeightZero"
          },
          {
            "name": "HeightHigherThanMax"
          },
          {
            "name": "PoseidonInvalidNumberOfInputs"
          },
          {
            "name": "PoseidonEmptyInput"
          },
          {
            "name": "PoseidonInvalidInputLength"
          },
          {
            "name": "PoseidonBytesToPrimeFieldElement"
          },
          {
            "name": "PoseidonInputLargerThanModulus"
          },
          {
            "name": "PoseidonVecToArray"
          },
          {
            "name": "PoseidonU64Tou8"
          },
          {
            "name": "PoseidonBytesToBigInt"
          },
          {
            "name": "PoseidonInvalidWidthCircom"
          },
          {
            "name": "PoseidonUnknown"
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
      "name": "UnshieldFailed",
      "msg": "UnshieldFailed"
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
      "msg": "Expected old Merkle trees as remaining accounts."
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

export const IDL: LightMerkleTreeProgram = {
  "version": "0.3.1",
  "name": "light_merkle_tree_program",
  "constants": [
    {
      "name": "ENCRYPTED_UTXOS_LENGTH",
      "type": {
        "defined": "usize"
      },
      "value": "174"
    },
    {
      "name": "MERKLE_TREE_HISTORY_SIZE",
      "type": "u64",
      "value": "256"
    },
    {
      "name": "MERKLE_TREE_HEIGHT",
      "type": {
        "defined": "usize"
      },
      "value": "18"
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
      "name": "ZERO_BYTES_MERKLE_TREE_18",
      "type": {
        "array": [
          {
            "array": [
              "u8",
              32
            ]
          },
          19
        ]
      },
      "value": "[[40 , 66 , 58 , 227 , 48 , 224 , 249 , 227 , 188 , 18 , 133 , 168 , 156 , 214 , 220 , 144 , 244 , 144 , 67 , 82 , 76 , 6 , 135 , 78 , 64 , 186 , 52 , 113 , 234 , 47 , 27 , 32] , [227 , 42 , 164 , 149 , 188 , 70 , 170 , 8 , 197 , 44 , 134 , 162 , 211 , 186 , 50 , 238 , 97 , 71 , 25 , 130 , 77 , 70 , 37 , 128 , 172 , 154 , 54 , 111 , 93 , 193 , 105 , 27] , [25 , 241 , 255 , 33 , 65 , 214 , 48 , 229 , 38 , 116 , 134 , 103 , 44 , 146 , 163 , 214 , 31 , 238 , 148 , 206 , 34 , 137 , 144 , 221 , 184 , 11 , 5 , 213 , 10 , 188 , 143 , 18] , [211 , 61 , 251 , 33 , 128 , 34 , 4 , 100 , 229 , 47 , 99 , 121 , 109 , 204 , 224 , 90 , 200 , 149 , 219 , 20 , 48 , 206 , 210 , 177 , 161 , 66 , 44 , 10 , 169 , 56 , 248 , 8] , [200 , 15 , 65 , 80 , 151 , 74 , 72 , 69 , 229 , 131 , 25 , 215 , 86 , 36 , 195 , 74 , 67 , 59 , 117 , 179 , 51 , 60 , 181 , 13 , 242 , 192 , 228 , 228 , 189 , 238 , 70 , 8] , [171 , 62 , 122 , 81 , 181 , 197 , 22 , 238 , 224 , 40 , 154 , 231 , 127 , 202 , 201 , 169 , 196 , 109 , 244 , 175 , 117 , 101 , 23 , 67 , 103 , 57 , 127 , 200 , 37 , 43 , 111 , 7] , [59 , 78 , 126 , 104 , 199 , 143 , 213 , 10 , 2 , 158 , 64 , 78 , 153 , 25 , 107 , 190 , 32 , 122 , 123 , 211 , 116 , 179 , 175 , 172 , 70 , 54 , 175 , 59 , 201 , 120 , 64 , 44] , [110 , 91 , 92 , 81 , 205 , 89 , 122 , 223 , 55 , 163 , 42 , 227 , 109 , 54 , 38 , 22 , 110 , 217 , 29 , 148 , 107 , 99 , 128 , 106 , 146 , 47 , 239 , 41 , 55 , 157 , 155 , 22] , [18 , 231 , 42 , 5 , 245 , 159 , 211 , 227 , 239 , 89 , 35 , 142 , 223 , 69 , 166 , 224 , 14 , 114 , 128 , 14 , 123 , 123 , 215 , 2 , 241 , 185 , 191 , 60 , 252 , 61 , 146 , 12] , [231 , 0 , 84 , 227 , 127 , 64 , 158 , 7 , 171 , 179 , 137 , 231 , 92 , 87 , 25 , 221 , 156 , 229 , 53 , 208 , 194 , 201 , 12 , 165 , 105 , 150 , 41 , 142 , 29 , 205 , 136 , 29] , [195 , 2 , 103 , 231 , 62 , 207 , 214 , 105 , 214 , 210 , 108 , 23 , 28 , 151 , 77 , 100 , 78 , 194 , 210 , 29 , 227 , 14 , 17 , 242 , 211 , 50 , 33 , 194 , 106 , 18 , 246 , 45] , [131 , 178 , 24 , 157 , 251 , 247 , 103 , 69 , 101 , 229 , 194 , 14 , 167 , 57 , 158 , 128 , 212 , 19 , 140 , 234 , 69 , 37 , 10 , 156 , 249 , 96 , 152 , 52 , 97 , 96 , 119 , 41] , [30 , 223 , 20 , 181 , 108 , 110 , 112 , 102 , 234 , 54 , 99 , 29 , 213 , 3 , 55 , 225 , 125 , 185 , 223 , 234 , 188 , 108 , 83 , 89 , 27 , 3 , 100 , 6 , 65 , 107 , 3 , 24] , [167 , 32 , 85 , 233 , 205 , 253 , 154 , 214 , 236 , 82 , 147 , 75 , 252 , 144 , 109 , 73 , 63 , 167 , 77 , 233 , 12 , 201 , 150 , 242 , 103 , 15 , 158 , 83 , 137 , 24 , 170 , 16] , [45 , 98 , 238 , 69 , 136 , 141 , 101 , 226 , 94 , 209 , 58 , 215 , 212 , 14 , 210 , 135 , 110 , 96 , 52 , 16 , 101 , 177 , 121 , 109 , 134 , 81 , 189 , 146 , 113 , 243 , 97 , 42] , [71 , 51 , 251 , 48 , 95 , 193 , 94 , 26 , 180 , 17 , 124 , 203 , 48 , 98 , 55 , 17 , 60 , 104 , 186 , 175 , 213 , 189 , 7 , 239 , 92 , 175 , 16 , 5 , 220 , 168 , 70 , 21] , [35 , 92 , 72 , 197 , 23 , 142 , 16 , 200 , 136 , 38 , 44 , 255 , 162 , 115 , 11 , 1 , 248 , 182 , 236 , 78 , 90 , 24 , 128 , 245 , 168 , 17 , 130 , 2 , 73 , 51 , 196 , 6] , [89 , 178 , 154 , 246 , 236 , 130 , 30 , 100 , 27 , 230 , 24 , 196 , 8 , 172 , 176 , 196 , 197 , 13 , 157 , 194 , 169 , 106 , 207 , 70 , 66 , 117 , 69 , 53 , 56 , 154 , 78 , 0] , [231 , 174 , 226 , 37 , 211 , 160 , 187 , 178 , 149 , 82 , 17 , 60 , 110 , 116 , 28 , 61 , 58 , 145 , 58 , 71 , 25 , 42 , 67 , 46 , 189 , 214 , 248 , 234 , 182 , 251 , 238 , 34]]"
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
    },
    {
      "name": "EVENT_MERKLE_TREE_SEED",
      "type": "bytes",
      "value": "[101, 118, 101, 110, 116, 95, 109, 101, 114, 107, 108, 101, 95, 116, 114, 101, 101]"
    },
    {
      "name": "TRANSACTION_MERKLE_TREE_SEED",
      "type": "bytes",
      "value": "[116, 114, 97, 110, 115, 97, 99, 116, 105, 111, 110, 95, 109, 101, 114, 107, 108, 101, 95, 116, 114, 101, 101]"
    }
  ],
  "instructions": [
    {
      "name": "initializeNewMerkleTrees",
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
          "name": "newTransactionMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "newEventMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "lockDuration",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initializeMerkleTreeAuthority",
      "docs": [
        "Initializes a new merkle tree authority which can register new verifiers and configure",
        "permissions to create new pools."
      ],
      "accounts": [
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "transactionMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "eventMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "updateMerkleTreeAuthority",
      "docs": [
        "Updates the merkle tree authority to a new authority."
      ],
      "accounts": [
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "newAuthority",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "enablePermissionlessSplTokens",
      "docs": [
        "Enables anyone to create token pools."
      ],
      "accounts": [
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "enablePermissionless",
          "type": "bool"
        }
      ]
    },
    {
      "name": "registerVerifier",
      "docs": [
        "Registers a new verifier which can unshield tokens, insert new nullifiers, add new leaves.",
        "These functions can only be invoked from registered verifiers."
      ],
      "accounts": [
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "verifierPubkey",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "registerPoolType",
      "docs": [
        "Registers a new pooltype."
      ],
      "accounts": [
        {
          "name": "registeredPoolTypePda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "poolType",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    },
    {
      "name": "registerSplPool",
      "docs": [
        "Creates a new spl token pool which can be used by any registered verifier."
      ],
      "accounts": [
        {
          "name": "registeredAssetPoolPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "merkleTreePdaToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "mint",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredPoolTypePda",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Just needs to exist and be derived correctly."
          ]
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "registerSolPool",
      "docs": [
        "Creates a new sol pool which can be used by any registered verifier."
      ],
      "accounts": [
        {
          "name": "registeredAssetPoolPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredPoolTypePda",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "insertTwoLeaves",
      "docs": [
        "Creates and initializes a pda which stores two merkle tree leaves and encrypted Utxos.",
        "The inserted leaves are not part of the Merkle tree yet and marked accordingly.",
        "The Merkle tree has to be updated after.",
        "Can only be called from a registered verifier program."
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "transactionMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": false,
          "isSigner": false
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
    },
    {
      "name": "insertTwoLeavesEvent",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "eventMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredVerifier",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "leafLeft",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "leafRight",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    },
    {
      "name": "unshieldSol",
      "docs": [
        "Unshields sol from a liquidity pool.",
        "An arbitrary number of recipients can be passed in with remaining accounts.",
        "Can only be called from a registered verifier program."
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTreeToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipient",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "unshieldSpl",
      "docs": [
        "Unshields spl tokens from a liquidity pool.",
        "An arbitrary number of recipients can be passed in with remaining accounts.",
        "Can only be called from a registered verifier program."
      ],
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTreeToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipient",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initializeNullifiers",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "nullifiers",
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
      "name": "registeredAssetPool",
      "docs": [
        "Nullfier pdas are derived from the nullifier",
        "existence of a nullifier is the check to prevent double spends."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "assetPoolPubkey",
            "type": "publicKey"
          },
          {
            "name": "poolType",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "index",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "registeredPoolType",
      "docs": [
        "Pool type"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "poolType",
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
      "name": "eventMerkleTree",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "merkleTreeNr",
            "type": "u64"
          },
          {
            "name": "newest",
            "type": "u8"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "merkleTree",
            "type": {
              "defined": "MerkleTree"
            }
          }
        ]
      }
    },
    {
      "name": "merkleTreePdaToken",
      "type": {
        "kind": "struct",
        "fields": []
      }
    },
    {
      "name": "preInsertedLeavesIndex",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nextIndex",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "merkleTreeAuthority",
      "docs": [
        "Configures the authority of the merkle tree which can:",
        "- register new verifiers",
        "- register new asset pools",
        "- register new asset pool types",
        "- set permissions for new asset pool creation",
        "- keeps current highest index for assets and merkle trees to enable lookups of these"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pubkey",
            "type": "publicKey"
          },
          {
            "name": "transactionMerkleTreeIndex",
            "type": "u64"
          },
          {
            "name": "eventMerkleTreeIndex",
            "type": "u64"
          },
          {
            "name": "registeredAssetIndex",
            "type": "u64"
          },
          {
            "name": "enablePermissionlessSplTokens",
            "type": "bool"
          },
          {
            "name": "enablePermissionlessMerkleTreeRegistration",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "merkleTreeUpdateState",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nodeLeft",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "nodeRight",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "leafLeft",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "leafRight",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "rpc",
            "type": "publicKey"
          },
          {
            "name": "merkleTreePdaPubkey",
            "type": "publicKey"
          },
          {
            "name": "state",
            "type": {
              "array": [
                "u8",
                96
              ]
            }
          },
          {
            "name": "currentRound",
            "type": "u64"
          },
          {
            "name": "currentRoundIndex",
            "type": "u64"
          },
          {
            "name": "currentInstructionIndex",
            "type": "u64"
          },
          {
            "name": "currentIndex",
            "type": "u64"
          },
          {
            "name": "currentLevel",
            "type": "u64"
          },
          {
            "name": "currentLevelHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "tmpLeavesIndex",
            "type": "u64"
          },
          {
            "name": "filledSubtrees",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                32
              ]
            }
          },
          {
            "name": "leaves",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        32
                      ]
                    },
                    2
                  ]
                },
                16
              ]
            }
          },
          {
            "name": "numberOfLeaves",
            "type": "u8"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "insertLeavesIndex",
            "type": "u8"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          }
        ]
      }
    },
    {
      "name": "registeredVerifier",
      "docs": [
        ""
      ],
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
      "name": "transactionMerkleTree",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "merkleTreeNr",
            "type": "u64"
          },
          {
            "name": "newest",
            "type": "u64"
          },
          {
            "name": "merkleTree",
            "type": {
              "defined": "MerkleTree"
            }
          }
        ]
      }
    },
    {
      "name": "twoLeavesBytesPda",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "nodeLeft",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "nodeRight",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "merkleTreePubkey",
            "type": "publicKey"
          },
          {
            "name": "leftLeafIndex",
            "type": "u64"
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "MerkleTree",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "height",
            "docs": [
              "Height of the Merkle tree."
            ],
            "type": "u64"
          },
          {
            "name": "filledSubtrees",
            "docs": [
              "Subtree hashes."
            ],
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                32
              ]
            }
          },
          {
            "name": "roots",
            "docs": [
              "Full history of roots of the Merkle tree (the last one is the current",
              "one)."
            ],
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                256
              ]
            }
          },
          {
            "name": "nextIndex",
            "docs": [
              "Next index to insert a leaf."
            ],
            "type": "u64"
          },
          {
            "name": "currentRootIndex",
            "docs": [
              "Current index of the root."
            ],
            "type": "u64"
          },
          {
            "name": "hashFunction",
            "docs": [
              "Hash implementation used on the Merkle tree."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "HashFunction",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Sha256"
          },
          {
            "name": "Poseidon"
          }
        ]
      }
    },
    {
      "name": "MerkleTreeError",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "HeightZero"
          },
          {
            "name": "HeightHigherThanMax"
          },
          {
            "name": "PoseidonInvalidNumberOfInputs"
          },
          {
            "name": "PoseidonEmptyInput"
          },
          {
            "name": "PoseidonInvalidInputLength"
          },
          {
            "name": "PoseidonBytesToPrimeFieldElement"
          },
          {
            "name": "PoseidonInputLargerThanModulus"
          },
          {
            "name": "PoseidonVecToArray"
          },
          {
            "name": "PoseidonU64Tou8"
          },
          {
            "name": "PoseidonBytesToBigInt"
          },
          {
            "name": "PoseidonInvalidWidthCircom"
          },
          {
            "name": "PoseidonUnknown"
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
      "name": "UnshieldFailed",
      "msg": "UnshieldFailed"
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
      "msg": "Expected old Merkle trees as remaining accounts."
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
