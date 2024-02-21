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
          "name": "authorityPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Mint authority, ensures that this program needs to be used as a proxy to mint tokens"
          ]
        },
        {
          "name": "mint",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredAssetPoolPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredPoolTypePda",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTreePdaToken",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "CHECK this account in merkle tree program"
          ]
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "compressedPdaProgram",
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
          "name": "merkleTreeAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "mint",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "merkleTreePdaToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "merkleTreeSet",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "noopProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTreeProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "accountCompressionProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "pspAccountCompressionAuthority",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "compressionPublicKeys",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
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
          "name": "signer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authorityPda",
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
          "name": "compressedPdaProgram",
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
      "name": "instructionDataTransfer",
      "type": {
        "kind": "struct",
        "fields": [
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
            "name": "lowElementIndexes",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "rootIndexes",
            "type": {
              "vec": "u64"
            }
          },
          {
            "name": "rpcFee",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "serializedUtxos",
            "type": {
              "defined": "SerializedUtxos"
            }
          }
        ]
      }
    }
  ],
  "types": [
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
          },
          {
            "name": "closeAuthority",
            "docs": [
              "Optional authority to close the account."
            ],
            "type": {
              "option": "publicKey"
            }
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
          "name": "authorityPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Mint authority, ensures that this program needs to be used as a proxy to mint tokens"
          ]
        },
        {
          "name": "mint",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredAssetPoolPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredPoolTypePda",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTreePdaToken",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "CHECK this account in merkle tree program"
          ]
        },
        {
          "name": "merkleTreeAuthorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "compressedPdaProgram",
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
          "name": "merkleTreeAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "mint",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "merkleTreePdaToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "merkleTreeSet",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "noopProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTreeProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "accountCompressionProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "pspAccountCompressionAuthority",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "compressionPublicKeys",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
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
          "name": "signer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authorityPda",
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
          "name": "compressedPdaProgram",
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
      "name": "instructionDataTransfer",
      "type": {
        "kind": "struct",
        "fields": [
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
            "name": "lowElementIndexes",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "rootIndexes",
            "type": {
              "vec": "u64"
            }
          },
          {
            "name": "rpcFee",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "serializedUtxos",
            "type": {
              "defined": "SerializedUtxos"
            }
          }
        ]
      }
    }
  ],
  "types": [
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
          },
          {
            "name": "closeAuthority",
            "docs": [
              "Optional authority to close the account."
            ],
            "type": {
              "option": "publicKey"
            }
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
    }
  ]
};
