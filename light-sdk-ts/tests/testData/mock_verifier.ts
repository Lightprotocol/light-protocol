export type MockVerifier = {
  "version": "0.1.0",
  "name": "mock_verifier",
  "instructions": [
    {
      "name": "shieldedTransferFirst",
      "docs": [
        "This instruction is the first step of a shieled transaction.",
        "It creates and initializes a verifier state account to save state of a verification during",
        "computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data",
        "such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic",
        "in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2"
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "First transaction, therefore the signing address is not checked but saved to be checked in future instructions."
          ]
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "publicAmountSpl",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "nullifiers",
          "type": {
            "array": [
              {
                "array": [
                  "u8",
                  32
                ]
              },
              4
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
              2
            ]
          }
        },
        {
          "name": "publicAmountSol",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "rootIndex",
          "type": "u64"
        },
        {
          "name": "relayerFee",
          "type": "u64"
        },
        {
          "name": "encryptedUtxos",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "shieldedTransferSecond",
      "docs": [
        "This instruction is the second step of a shieled transaction.",
        "The proof is verified with the parameters saved in the first transaction.",
        "At successful verification protocol logic is executed."
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "programMerkleTree",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "transactionMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "senderSpl",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientSpl",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "senderSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "relayerRecipientSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "verifierProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "proofAApp",
          "type": {
            "array": [
              "u8",
              64
            ]
          }
        },
        {
          "name": "proofBApp",
          "type": {
            "array": [
              "u8",
              128
            ]
          }
        },
        {
          "name": "proofCApp",
          "type": {
            "array": [
              "u8",
              64
            ]
          }
        },
        {
          "name": "proofAVerifier",
          "type": {
            "array": [
              "u8",
              64
            ]
          }
        },
        {
          "name": "proofBVerifier",
          "type": {
            "array": [
              "u8",
              128
            ]
          }
        },
        {
          "name": "proofCVerifier",
          "type": {
            "array": [
              "u8",
              64
            ]
          }
        },
        {
          "name": "transactionHash",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "closeVerifierState",
      "docs": [
        "Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify."
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "u256",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "x",
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
      "name": "utxo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "amounts",
            "type": {
              "array": [
                "u64",
                2
              ]
            }
          },
          {
            "name": "splAssetIndex",
            "type": "u64"
          },
          {
            "name": "blinding",
            "type": "u256"
          },
          {
            "name": "verifierAddress",
            "type": "publicKey"
          },
          {
            "name": "testInput1",
            "type": "u256"
          },
          {
            "name": "testInput2",
            "type": "u256"
          }
        ]
      }
    },
    {
      "name": "utxoAppData",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "testInput1",
            "type": "u256"
          },
          {
            "name": "testInput2",
            "type": "u256"
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "OfferExpired",
      "msg": "The offer expired."
    }
  ]
};

export const IDL: MockVerifier = {
  "version": "0.1.0",
  "name": "mock_verifier",
  "instructions": [
    {
      "name": "shieldedTransferFirst",
      "docs": [
        "This instruction is the first step of a shieled transaction.",
        "It creates and initializes a verifier state account to save state of a verification during",
        "computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data",
        "such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic",
        "in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2"
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "First transaction, therefore the signing address is not checked but saved to be checked in future instructions."
          ]
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "publicAmountSpl",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "nullifiers",
          "type": {
            "array": [
              {
                "array": [
                  "u8",
                  32
                ]
              },
              4
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
              2
            ]
          }
        },
        {
          "name": "publicAmountSol",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "rootIndex",
          "type": "u64"
        },
        {
          "name": "relayerFee",
          "type": "u64"
        },
        {
          "name": "encryptedUtxos",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "shieldedTransferSecond",
      "docs": [
        "This instruction is the second step of a shieled transaction.",
        "The proof is verified with the parameters saved in the first transaction.",
        "At successful verification protocol logic is executed."
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "programMerkleTree",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "transactionMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "senderSpl",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientSpl",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "senderSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "relayerRecipientSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "verifierProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "proofAApp",
          "type": {
            "array": [
              "u8",
              64
            ]
          }
        },
        {
          "name": "proofBApp",
          "type": {
            "array": [
              "u8",
              128
            ]
          }
        },
        {
          "name": "proofCApp",
          "type": {
            "array": [
              "u8",
              64
            ]
          }
        },
        {
          "name": "proofAVerifier",
          "type": {
            "array": [
              "u8",
              64
            ]
          }
        },
        {
          "name": "proofBVerifier",
          "type": {
            "array": [
              "u8",
              128
            ]
          }
        },
        {
          "name": "proofCVerifier",
          "type": {
            "array": [
              "u8",
              64
            ]
          }
        },
        {
          "name": "transactionHash",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "closeVerifierState",
      "docs": [
        "Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify."
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "u256",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "x",
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
      "name": "utxo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "amounts",
            "type": {
              "array": [
                "u64",
                2
              ]
            }
          },
          {
            "name": "splAssetIndex",
            "type": "u64"
          },
          {
            "name": "blinding",
            "type": "u256"
          },
          {
            "name": "verifierAddress",
            "type": "publicKey"
          },
          {
            "name": "testInput1",
            "type": "u256"
          },
          {
            "name": "testInput2",
            "type": "u256"
          }
        ]
      }
    },
    {
      "name": "utxoAppData",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "testInput1",
            "type": "u256"
          },
          {
            "name": "testInput2",
            "type": "u256"
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "OfferExpired",
      "msg": "The offer expired."
    }
  ]
};
