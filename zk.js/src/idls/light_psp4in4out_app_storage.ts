export type LightPsp4in4outAppStorage = {
  "version": "0.3.1",
  "name": "light_psp4in4out_app_storage",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86\""
    }
  ],
  "instructions": [
    {
      "name": "compressedTransferInputs",
      "docs": [
        "This instruction is used to invoke this system verifier and can only be invoked via cpi."
      ],
      "accounts": [
        {
          "name": "verifierState",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "proofA",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "proofB",
          "type": {
            "array": [
              "u8",
              64
            ]
          }
        },
        {
          "name": "proofC",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "connectingHash",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "startOffset",
          "type": {
            "defined": "usize"
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "zKprivateProgramTransaction4In4OutMainProofInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicOutUtxoHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicProgramId",
            "type": "u8"
          },
          {
            "name": "publicTransactionHash",
            "type": "u8"
          },
          {
            "name": "publicNewAddress",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "assetPublicKeys",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "privatePublicDataHash",
            "type": "u8"
          },
          {
            "name": "isInProgramUtxo",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inOwner",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "inPrivateKey",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inBlinding",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inDataHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "leafIndex",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "merkleProof",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    22
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "inIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "nullifierLeafIndex",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "nullifierMerkleProof",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    22
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outOwner",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outBlinding",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outDataHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "metaHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inAddress",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "isInAddress",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "isNewAddress",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          }
        ]
      }
    },
    {
      "name": "zKprivateProgramTransaction4In4OutMainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicOutUtxoHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicProgramId",
            "type": "u8"
          },
          {
            "name": "publicTransactionHash",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "instructionDataLightInstructionPrivateProgramTransaction4In4OutMainSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
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
            "name": "publicNullifierRoot",
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
            "name": "publicAmountSpl",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicDataHash",
            "type": {
              "array": [
                "u8",
                32
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
            "name": "publicMintPublicKey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicNullifier",
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
            "name": "publicOutUtxoHash",
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
            "name": "publicProgramId",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicTransactionHash",
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
      "name": "psp4In4OutAppStorageVerifierState",
      "type": {
        "kind": "struct",
        "fields": [
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
                    "u8",
                    32
                  ]
                },
                4
              ]
            }
          },
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
            "name": "publicAmountSol",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "rpcFee",
            "type": "u64"
          },
          {
            "name": "encryptedUtxos",
            "type": {
              "array": [
                "u8",
                512
              ]
            }
          },
          {
            "name": "merkleRootIndex",
            "type": "u64"
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "InvalidVerifier",
      "msg": "System program is no valid verifier."
    }
  ]
};

export const IDL: LightPsp4in4outAppStorage = {
  "version": "0.3.1",
  "name": "light_psp4in4out_app_storage",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86\""
    }
  ],
  "instructions": [
    {
      "name": "compressedTransferInputs",
      "docs": [
        "This instruction is used to invoke this system verifier and can only be invoked via cpi."
      ],
      "accounts": [
        {
          "name": "verifierState",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "proofA",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "proofB",
          "type": {
            "array": [
              "u8",
              64
            ]
          }
        },
        {
          "name": "proofC",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "connectingHash",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "startOffset",
          "type": {
            "defined": "usize"
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "zKprivateProgramTransaction4In4OutMainProofInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicOutUtxoHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicProgramId",
            "type": "u8"
          },
          {
            "name": "publicTransactionHash",
            "type": "u8"
          },
          {
            "name": "publicNewAddress",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "assetPublicKeys",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "privatePublicDataHash",
            "type": "u8"
          },
          {
            "name": "isInProgramUtxo",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inOwner",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "inPrivateKey",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inBlinding",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inDataHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "leafIndex",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "merkleProof",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    22
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "inIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "nullifierLeafIndex",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "nullifierMerkleProof",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    22
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outOwner",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outBlinding",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outDataHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "metaHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inAddress",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "isInAddress",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "isNewAddress",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          }
        ]
      }
    },
    {
      "name": "zKprivateProgramTransaction4In4OutMainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicOutUtxoHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "publicProgramId",
            "type": "u8"
          },
          {
            "name": "publicTransactionHash",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "instructionDataLightInstructionPrivateProgramTransaction4In4OutMainSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
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
            "name": "publicNullifierRoot",
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
            "name": "publicAmountSpl",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicDataHash",
            "type": {
              "array": [
                "u8",
                32
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
            "name": "publicMintPublicKey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicNullifier",
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
            "name": "publicOutUtxoHash",
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
            "name": "publicProgramId",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicTransactionHash",
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
      "name": "psp4In4OutAppStorageVerifierState",
      "type": {
        "kind": "struct",
        "fields": [
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
                    "u8",
                    32
                  ]
                },
                4
              ]
            }
          },
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
            "name": "publicAmountSol",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "rpcFee",
            "type": "u64"
          },
          {
            "name": "encryptedUtxos",
            "type": {
              "array": [
                "u8",
                512
              ]
            }
          },
          {
            "name": "merkleRootIndex",
            "type": "u64"
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "InvalidVerifier",
      "msg": "System program is no valid verifier."
    }
  ]
};
