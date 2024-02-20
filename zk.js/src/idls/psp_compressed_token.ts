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
      "name": "CpiSignatureAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "signatures",
            "type": {
              "vec": {
                "defined": "CpiSignature"
              }
            }
          }
        ]
      }
    },
    {
      "name": "CpiSignature",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "program",
            "type": "publicKey"
          },
          {
            "name": "tlvHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "tlvData",
            "type": {
              "defined": "TlvDataElement"
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
            "name": "lowElementIndices",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "rootIndices",
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
            "name": "inUtxos",
            "type": {
              "vec": {
                "defined": "Utxo"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "OutUtxo"
              }
            }
          },
          {
            "name": "inUtxoMerkleTreeRemainingAccountIndex",
            "type": "bytes"
          },
          {
            "name": "inUtxoNullifierQueueRemainingAccountIndex",
            "type": "bytes"
          },
          {
            "name": "outUtxoMerkleTreeRemainingAccountIndex",
            "type": "bytes"
          }
        ]
      }
    },
    {
      "name": "InstructionDataTransfer2",
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
            "name": "lowElementIndices",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "rootIndices",
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
            "name": "utxos",
            "type": {
              "defined": "SerializedUtxos"
            }
          },
          {
            "name": "inUtxoMerkleTreeRemainingAccountIndex",
            "type": "bytes"
          },
          {
            "name": "inUtxoNullifierQueueRemainingAccountIndex",
            "type": "bytes"
          },
          {
            "name": "outUtxoMerkleTreeRemainingAccountIndex",
            "type": "bytes"
          }
        ]
      }
    },
    {
      "name": "SerializedUtxos",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pubkeyArray",
            "type": {
              "vec": "publicKey"
            }
          },
          {
            "name": "u64Array",
            "type": {
              "vec": "u64"
            }
          },
          {
            "name": "inUtxos",
            "type": {
              "vec": {
                "defined": "InUtxoSerializable"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "OutUtxoSerializable"
              }
            }
          }
        ]
      }
    },
    {
      "name": "InUtxoSerializable",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "u8"
          },
          {
            "name": "leafIndex",
            "type": "u32"
          },
          {
            "name": "lamports",
            "type": "u8"
          },
          {
            "name": "data",
            "type": {
              "option": {
                "defined": "TlvSerializable"
              }
            }
          }
        ]
      }
    },
    {
      "name": "OutUtxoSerializable",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "u8"
          },
          {
            "name": "lamports",
            "type": "u8"
          },
          {
            "name": "data",
            "type": {
              "option": {
                "defined": "TlvSerializable"
              }
            }
          }
        ]
      }
    },
    {
      "name": "OutUtxo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "publicKey"
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
      "name": "Utxo",
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
      "name": "TlvSerializable",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tlvElements",
            "type": {
              "vec": {
                "defined": "TlvDataElementSerializable"
              }
            }
          }
        ]
      }
    },
    {
      "name": "Tlv",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tlvElements",
            "type": {
              "vec": {
                "defined": "TlvDataElement"
              }
            }
          }
        ]
      }
    },
    {
      "name": "TlvDataElementSerializable",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "discriminator",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "owner",
            "type": "u8"
          },
          {
            "name": "data",
            "type": "bytes"
          },
          {
            "name": "dataHash",
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
      "name": "TlvDataElement",
      "docs": [
        "Time lock escrow example:",
        "escrow tlv data -> compressed token program",
        "let escrow_data = {",
        "owner: Pubkey, // owner is the user pubkey",
        "release_slot: u64,",
        "deposit_slot: u64,",
        "};",
        "",
        "let escrow_tlv_data = TlvDataElement {",
        "discriminator: [1,0,0,0,0,0,0,0],",
        "owner: escrow_program_id,",
        "data: escrow_data.try_to_vec()?,",
        "};",
        "let token_tlv = TlvDataElement {",
        "discriminator: [2,0,0,0,0,0,0,0],",
        "owner: token_program,",
        "data: token_data.try_to_vec()?,",
        "};",
        "let token_data = Account {",
        "mint,",
        "owner,",
        "amount: 10_000_000u64,",
        "delegate: None,",
        "state: Initialized, (u64)",
        "is_native: None,",
        "delegated_amount: 0u64,",
        "close_authority: None,",
        "};",
        ""
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "discriminator",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "data",
            "type": "bytes"
          },
          {
            "name": "dataHash",
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
      "name": "ErrorCode",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "SumCheckFailed"
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
      "name": "CpiSignatureAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "signatures",
            "type": {
              "vec": {
                "defined": "CpiSignature"
              }
            }
          }
        ]
      }
    },
    {
      "name": "CpiSignature",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "program",
            "type": "publicKey"
          },
          {
            "name": "tlvHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "tlvData",
            "type": {
              "defined": "TlvDataElement"
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
            "name": "lowElementIndices",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "rootIndices",
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
            "name": "inUtxos",
            "type": {
              "vec": {
                "defined": "Utxo"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "OutUtxo"
              }
            }
          },
          {
            "name": "inUtxoMerkleTreeRemainingAccountIndex",
            "type": "bytes"
          },
          {
            "name": "inUtxoNullifierQueueRemainingAccountIndex",
            "type": "bytes"
          },
          {
            "name": "outUtxoMerkleTreeRemainingAccountIndex",
            "type": "bytes"
          }
        ]
      }
    },
    {
      "name": "InstructionDataTransfer2",
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
            "name": "lowElementIndices",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "rootIndices",
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
            "name": "utxos",
            "type": {
              "defined": "SerializedUtxos"
            }
          },
          {
            "name": "inUtxoMerkleTreeRemainingAccountIndex",
            "type": "bytes"
          },
          {
            "name": "inUtxoNullifierQueueRemainingAccountIndex",
            "type": "bytes"
          },
          {
            "name": "outUtxoMerkleTreeRemainingAccountIndex",
            "type": "bytes"
          }
        ]
      }
    },
    {
      "name": "SerializedUtxos",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pubkeyArray",
            "type": {
              "vec": "publicKey"
            }
          },
          {
            "name": "u64Array",
            "type": {
              "vec": "u64"
            }
          },
          {
            "name": "inUtxos",
            "type": {
              "vec": {
                "defined": "InUtxoSerializable"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "OutUtxoSerializable"
              }
            }
          }
        ]
      }
    },
    {
      "name": "InUtxoSerializable",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "u8"
          },
          {
            "name": "leafIndex",
            "type": "u32"
          },
          {
            "name": "lamports",
            "type": "u8"
          },
          {
            "name": "data",
            "type": {
              "option": {
                "defined": "TlvSerializable"
              }
            }
          }
        ]
      }
    },
    {
      "name": "OutUtxoSerializable",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "u8"
          },
          {
            "name": "lamports",
            "type": "u8"
          },
          {
            "name": "data",
            "type": {
              "option": {
                "defined": "TlvSerializable"
              }
            }
          }
        ]
      }
    },
    {
      "name": "OutUtxo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "publicKey"
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
      "name": "Utxo",
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
      "name": "TlvSerializable",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tlvElements",
            "type": {
              "vec": {
                "defined": "TlvDataElementSerializable"
              }
            }
          }
        ]
      }
    },
    {
      "name": "Tlv",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tlvElements",
            "type": {
              "vec": {
                "defined": "TlvDataElement"
              }
            }
          }
        ]
      }
    },
    {
      "name": "TlvDataElementSerializable",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "discriminator",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "owner",
            "type": "u8"
          },
          {
            "name": "data",
            "type": "bytes"
          },
          {
            "name": "dataHash",
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
      "name": "TlvDataElement",
      "docs": [
        "Time lock escrow example:",
        "escrow tlv data -> compressed token program",
        "let escrow_data = {",
        "owner: Pubkey, // owner is the user pubkey",
        "release_slot: u64,",
        "deposit_slot: u64,",
        "};",
        "",
        "let escrow_tlv_data = TlvDataElement {",
        "discriminator: [1,0,0,0,0,0,0,0],",
        "owner: escrow_program_id,",
        "data: escrow_data.try_to_vec()?,",
        "};",
        "let token_tlv = TlvDataElement {",
        "discriminator: [2,0,0,0,0,0,0,0],",
        "owner: token_program,",
        "data: token_data.try_to_vec()?,",
        "};",
        "let token_data = Account {",
        "mint,",
        "owner,",
        "amount: 10_000_000u64,",
        "delegate: None,",
        "state: Initialized, (u64)",
        "is_native: None,",
        "delegated_amount: 0u64,",
        "close_authority: None,",
        "};",
        ""
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "discriminator",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "data",
            "type": "bytes"
          },
          {
            "name": "dataHash",
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
      "name": "ErrorCode",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "SumCheckFailed"
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
