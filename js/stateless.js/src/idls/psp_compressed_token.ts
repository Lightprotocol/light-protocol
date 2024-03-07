export type PspCompressedToken = {
  "version": "0.3.0",
  "name": "psp_compressed_token",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE\""
    }
  ],
  "instructions": [
    {
      "name": "createMint",
      "docs": [
        "This instruction expects a mint account to be created in a separate token program instruction",
        "with token authority as mint authority.",
        "This instruction creates a token pool account for that mint owned by token authority."
      ],
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenPoolPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "mint",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "mintAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "mintTo",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "mintAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "mint",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenPoolPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "compressedPdaProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredProgramPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "noopProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "pspAccountCompressionAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "accountCompressionProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "publicKeys",
          "type": {
            "vec": "publicKey"
          }
        },
        {
          "name": "amounts",
          "type": {
            "vec": "u64"
          }
        }
      ]
    },
    {
      "name": "transfer",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "cpiAuthorityPda",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "compressedPdaProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredProgramPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "noopProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "pspAccountCompressionAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "accountCompressionProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "selfProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "instructionDataTransferClient",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proof",
            "type": {
              "option": {
                "defined": "CompressedProofClient"
              }
            }
          },
          {
            "name": "rootIndices",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "inUtxos",
            "type": {
              "vec": {
                "defined": "InUtxoTupleClient"
              }
            }
          },
          {
            "name": "inTlvData",
            "type": {
              "vec": {
                "defined": "TokenTlvData"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "TokenTransferOutUtxo"
              }
            }
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "UtxoClient",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "blinding",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "lamports",
            "type": "u64"
          },
          {
            "name": "data",
            "type": {
              "option": {
                "defined": "Tlv"
              }
            }
          }
        ]
      }
    },
    {
      "name": "InUtxoTupleClient",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "inUtxo",
            "type": {
              "defined": "UtxoClient"
            }
          },
          {
            "name": "indexMtAccount",
            "type": "u8"
          },
          {
            "name": "indexNullifierArrayAccount",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "CompressedProofClient",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "a",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "b",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "c",
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
      "name": "InstructionDataTransfer",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proof",
            "type": {
              "option": {
                "defined": "CompressedProof"
              }
            }
          },
          {
            "name": "rootIndices",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "inUtxos",
            "type": {
              "vec": {
                "defined": "InUtxoTuple"
              }
            }
          },
          {
            "name": "inTlvData",
            "type": {
              "vec": {
                "defined": "TokenTlvData"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "TokenTransferOutUtxo"
              }
            }
          }
        ]
      }
    },
    {
      "name": "TokenTransferOutUtxo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "lamports",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "indexMtAccount",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "TokenTlvData",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The mint associated with this account"
            ],
            "type": "publicKey"
          },
          {
            "name": "owner",
            "docs": [
              "The owner of this account."
            ],
            "type": "publicKey"
          },
          {
            "name": "amount",
            "docs": [
              "The amount of tokens this account holds."
            ],
            "type": "u64"
          },
          {
            "name": "delegate",
            "docs": [
              "If `delegate` is `Some` then `delegated_amount` represents",
              "the amount authorized by the delegate"
            ],
            "type": {
              "option": "publicKey"
            }
          },
          {
            "name": "state",
            "docs": [
              "The account's state"
            ],
            "type": {
              "defined": "AccountState"
            }
          },
          {
            "name": "isNative",
            "docs": [
              "If is_some, this is a native token, and the value logs the rent-exempt",
              "reserve. An Account is required to be rent-exempt, so the value is",
              "used by the Processor to ensure that wrapped SOL accounts do not",
              "drop below this threshold."
            ],
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "delegatedAmount",
            "docs": [
              "The amount delegated"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "AccountState",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Uninitialized"
          },
          {
            "name": "Initialized"
          },
          {
            "name": "Frozen"
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "PublicKeyAmountMissmatch",
      "msg": "public keys and amounts must be of same length"
    },
    {
      "code": 6001,
      "name": "MissingNewAuthorityPda",
      "msg": "missing new authority pda"
    },
    {
      "code": 6002,
      "name": "SignerCheckFailed",
      "msg": "SignerCheckFailed"
    },
    {
      "code": 6003,
      "name": "MintCheckFailed",
      "msg": "MintCheckFailed"
    },
    {
      "code": 6004,
      "name": "ComputeInputSumFailed",
      "msg": "ComputeInputSumFailed"
    },
    {
      "code": 6005,
      "name": "ComputeOutputSumFailed",
      "msg": "ComputeOutputSumFailed"
    },
    {
      "code": 6006,
      "name": "ComputeCompressSumFailed",
      "msg": "ComputeCompressSumFailed"
    },
    {
      "code": 6007,
      "name": "ComputeDecompressSumFailed",
      "msg": "ComputeDecompressSumFailed"
    },
    {
      "code": 6008,
      "name": "SumCheckFailed",
      "msg": "SumCheckFailed"
    }
  ]
};

export const IDL: PspCompressedToken = {
  "version": "0.3.0",
  "name": "psp_compressed_token",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE\""
    }
  ],
  "instructions": [
    {
      "name": "createMint",
      "docs": [
        "This instruction expects a mint account to be created in a separate token program instruction",
        "with token authority as mint authority.",
        "This instruction creates a token pool account for that mint owned by token authority."
      ],
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenPoolPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "mint",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "mintAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "mintTo",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "mintAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "mint",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenPoolPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "compressedPdaProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredProgramPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "noopProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "pspAccountCompressionAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "accountCompressionProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "publicKeys",
          "type": {
            "vec": "publicKey"
          }
        },
        {
          "name": "amounts",
          "type": {
            "vec": "u64"
          }
        }
      ]
    },
    {
      "name": "transfer",
      "accounts": [
        {
          "name": "feePayer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "cpiAuthorityPda",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "compressedPdaProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredProgramPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "noopProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "pspAccountCompressionAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "accountCompressionProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "selfProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "instructionDataTransferClient",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proof",
            "type": {
              "option": {
                "defined": "CompressedProofClient"
              }
            }
          },
          {
            "name": "rootIndices",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "inUtxos",
            "type": {
              "vec": {
                "defined": "InUtxoTupleClient"
              }
            }
          },
          {
            "name": "inTlvData",
            "type": {
              "vec": {
                "defined": "TokenTlvData"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "TokenTransferOutUtxo"
              }
            }
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "UtxoClient",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "blinding",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "lamports",
            "type": "u64"
          },
          {
            "name": "data",
            "type": {
              "option": {
                "defined": "Tlv"
              }
            }
          }
        ]
      }
    },
    {
      "name": "InUtxoTupleClient",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "inUtxo",
            "type": {
              "defined": "UtxoClient"
            }
          },
          {
            "name": "indexMtAccount",
            "type": "u8"
          },
          {
            "name": "indexNullifierArrayAccount",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "CompressedProofClient",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "a",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "b",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "c",
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
      "name": "InstructionDataTransfer",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proof",
            "type": {
              "option": {
                "defined": "CompressedProof"
              }
            }
          },
          {
            "name": "rootIndices",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "inUtxos",
            "type": {
              "vec": {
                "defined": "InUtxoTuple"
              }
            }
          },
          {
            "name": "inTlvData",
            "type": {
              "vec": {
                "defined": "TokenTlvData"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "TokenTransferOutUtxo"
              }
            }
          }
        ]
      }
    },
    {
      "name": "TokenTransferOutUtxo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "lamports",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "indexMtAccount",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "TokenTlvData",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The mint associated with this account"
            ],
            "type": "publicKey"
          },
          {
            "name": "owner",
            "docs": [
              "The owner of this account."
            ],
            "type": "publicKey"
          },
          {
            "name": "amount",
            "docs": [
              "The amount of tokens this account holds."
            ],
            "type": "u64"
          },
          {
            "name": "delegate",
            "docs": [
              "If `delegate` is `Some` then `delegated_amount` represents",
              "the amount authorized by the delegate"
            ],
            "type": {
              "option": "publicKey"
            }
          },
          {
            "name": "state",
            "docs": [
              "The account's state"
            ],
            "type": {
              "defined": "AccountState"
            }
          },
          {
            "name": "isNative",
            "docs": [
              "If is_some, this is a native token, and the value logs the rent-exempt",
              "reserve. An Account is required to be rent-exempt, so the value is",
              "used by the Processor to ensure that wrapped SOL accounts do not",
              "drop below this threshold."
            ],
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "delegatedAmount",
            "docs": [
              "The amount delegated"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "AccountState",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Uninitialized"
          },
          {
            "name": "Initialized"
          },
          {
            "name": "Frozen"
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "PublicKeyAmountMissmatch",
      "msg": "public keys and amounts must be of same length"
    },
    {
      "code": 6001,
      "name": "MissingNewAuthorityPda",
      "msg": "missing new authority pda"
    },
    {
      "code": 6002,
      "name": "SignerCheckFailed",
      "msg": "SignerCheckFailed"
    },
    {
      "code": 6003,
      "name": "MintCheckFailed",
      "msg": "MintCheckFailed"
    },
    {
      "code": 6004,
      "name": "ComputeInputSumFailed",
      "msg": "ComputeInputSumFailed"
    },
    {
      "code": 6005,
      "name": "ComputeOutputSumFailed",
      "msg": "ComputeOutputSumFailed"
    },
    {
      "code": 6006,
      "name": "ComputeCompressSumFailed",
      "msg": "ComputeCompressSumFailed"
    },
    {
      "code": 6007,
      "name": "ComputeDecompressSumFailed",
      "msg": "ComputeDecompressSumFailed"
    },
    {
      "code": 6008,
      "name": "SumCheckFailed",
      "msg": "SumCheckFailed"
    }
  ]
};
