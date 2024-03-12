export type PspCompressedPda = {
  "version": "0.3.0",
  "name": "psp_compressed_pda",
  "instructions": [
    {
      "name": "executeCompressedTransaction",
      "docs": [
        "This function can be used to transfer sol and execute any other compressed transaction.",
        "Instruction data is not optimized for space.",
        "This method can be called by cpi so that instruction data can be compressed with a custom algorithm."
      ],
      "accounts": [
        {
          "name": "signer",
          "isMut": false,
          "isSigner": true
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
          "name": "cpiSignatureAccount",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "invokingProgram",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ],
      "returns": {
        "defined": "crate::event::PublicTransactionEvent"
      }
    },
    {
      "name": "executeCompressedTransaction2",
      "docs": [
        "This function can be used to transfer sol and execute any other compressed transaction.",
        "Instruction data is optimized for space."
      ],
      "accounts": [
        {
          "name": "signer",
          "isMut": false,
          "isSigner": true
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
          "name": "cpiSignatureAccount",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "invokingProgram",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ],
      "returns": {
        "defined": "crate::event::PublicTransactionEvent"
      }
    }
  ],
  "accounts": [
    {
      "name": "cpiSignatureAccount",
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
      "name": "instructionDataTransfer",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proof",
            "type": {
              "option": {
                "defined": "ProofCompressed"
              }
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
              "vec": "u16"
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
                "defined": "InUtxoTuple"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "OutUtxoTuple"
              }
            }
          }
        ]
      }
    },
    {
      "name": "instructionDataTransfer2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proof",
            "type": {
              "option": {
                "defined": "ProofCompressed"
              }
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
              "vec": "u16"
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
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "PublicTransactionEvent",
      "type": {
        "kind": "struct",
        "fields": [
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
                "defined": "Utxo"
              }
            }
          },
          {
            "name": "outUtxoIndices",
            "type": {
              "vec": "u64"
            }
          },
          {
            "name": "deCompressAmount",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "rpcFee",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "message",
            "type": {
              "option": "bytes"
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
      "name": "CompressedProof",
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
      "name": "InUtxoSerializableTuple",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "inUtxoSerializable",
            "type": {
              "defined": "InUtxoSerializable"
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
      "name": "OutUtxoSerializableTuple",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "outUtxoSerializable",
            "type": {
              "defined": "OutUtxoSerializable"
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
      "name": "InUtxoTuple",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "inUtxo",
            "type": {
              "defined": "Utxo"
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
      "name": "OutUtxoTuple",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "outUtxo",
            "type": {
              "defined": "OutUtxo"
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
                "defined": "InUtxoSerializableTuple"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "OutUtxoSerializableTuple"
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
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "SumCheckFailed",
      "msg": "Sum check failed"
    },
    {
      "code": 6001,
      "name": "SignerCheckFailed",
      "msg": "Signer check failed"
    },
    {
      "code": 6002,
      "name": "CpiSignerCheckFailed",
      "msg": "Cpi signer check failed"
    },
    {
      "code": 6003,
      "name": "ComputeInputSumFailed",
      "msg": "Computing input sum failed."
    },
    {
      "code": 6004,
      "name": "ComputeOutputSumFailed",
      "msg": "Computing output sum failed."
    },
    {
      "code": 6005,
      "name": "ComputeRpcSumFailed",
      "msg": "Computing rpc sum failed."
    },
    {
      "code": 6006,
      "name": "InUtxosAlreadyAdded",
      "msg": "InUtxosAlreadyAdded"
    },
    {
      "code": 6007,
      "name": "NumberOfLeavesMissmatch",
      "msg": "NumberOfLeavesMissmatch"
    },
    {
      "code": 6008,
      "name": "MerkleTreePubkeysMissmatch",
      "msg": "MerkleTreePubkeysMissmatch"
    },
    {
      "code": 6009,
      "name": "NullifierArrayPubkeysMissmatch",
      "msg": "NullifierArrayPubkeysMissmatch"
    },
    {
      "code": 6010,
      "name": "InvalidNoopPubkey",
      "msg": "InvalidNoopPubkey"
    }
  ]
};

export const IDL: PspCompressedPda = {
  "version": "0.3.0",
  "name": "psp_compressed_pda",
  "instructions": [
    {
      "name": "executeCompressedTransaction",
      "docs": [
        "This function can be used to transfer sol and execute any other compressed transaction.",
        "Instruction data is not optimized for space.",
        "This method can be called by cpi so that instruction data can be compressed with a custom algorithm."
      ],
      "accounts": [
        {
          "name": "signer",
          "isMut": false,
          "isSigner": true
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
          "name": "cpiSignatureAccount",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "invokingProgram",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ],
      "returns": {
        "defined": "crate::event::PublicTransactionEvent"
      }
    },
    {
      "name": "executeCompressedTransaction2",
      "docs": [
        "This function can be used to transfer sol and execute any other compressed transaction.",
        "Instruction data is optimized for space."
      ],
      "accounts": [
        {
          "name": "signer",
          "isMut": false,
          "isSigner": true
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
          "name": "cpiSignatureAccount",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        },
        {
          "name": "invokingProgram",
          "isMut": false,
          "isSigner": false,
          "isOptional": true
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ],
      "returns": {
        "defined": "crate::event::PublicTransactionEvent"
      }
    }
  ],
  "accounts": [
    {
      "name": "cpiSignatureAccount",
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
      "name": "instructionDataTransfer",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proof",
            "type": {
              "option": {
                "defined": "ProofCompressed"
              }
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
              "vec": "u16"
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
                "defined": "InUtxoTuple"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "OutUtxoTuple"
              }
            }
          }
        ]
      }
    },
    {
      "name": "instructionDataTransfer2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proof",
            "type": {
              "option": {
                "defined": "ProofCompressed"
              }
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
              "vec": "u16"
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
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "PublicTransactionEvent",
      "type": {
        "kind": "struct",
        "fields": [
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
                "defined": "Utxo"
              }
            }
          },
          {
            "name": "outUtxoIndices",
            "type": {
              "vec": "u64"
            }
          },
          {
            "name": "deCompressAmount",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "rpcFee",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "message",
            "type": {
              "option": "bytes"
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
      "name": "CompressedProof",
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
      "name": "InUtxoSerializableTuple",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "inUtxoSerializable",
            "type": {
              "defined": "InUtxoSerializable"
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
      "name": "OutUtxoSerializableTuple",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "outUtxoSerializable",
            "type": {
              "defined": "OutUtxoSerializable"
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
      "name": "InUtxoTuple",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "inUtxo",
            "type": {
              "defined": "Utxo"
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
      "name": "OutUtxoTuple",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "outUtxo",
            "type": {
              "defined": "OutUtxo"
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
                "defined": "InUtxoSerializableTuple"
              }
            }
          },
          {
            "name": "outUtxos",
            "type": {
              "vec": {
                "defined": "OutUtxoSerializableTuple"
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
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "SumCheckFailed",
      "msg": "Sum check failed"
    },
    {
      "code": 6001,
      "name": "SignerCheckFailed",
      "msg": "Signer check failed"
    },
    {
      "code": 6002,
      "name": "CpiSignerCheckFailed",
      "msg": "Cpi signer check failed"
    },
    {
      "code": 6003,
      "name": "ComputeInputSumFailed",
      "msg": "Computing input sum failed."
    },
    {
      "code": 6004,
      "name": "ComputeOutputSumFailed",
      "msg": "Computing output sum failed."
    },
    {
      "code": 6005,
      "name": "ComputeRpcSumFailed",
      "msg": "Computing rpc sum failed."
    },
    {
      "code": 6006,
      "name": "InUtxosAlreadyAdded",
      "msg": "InUtxosAlreadyAdded"
    },
    {
      "code": 6007,
      "name": "NumberOfLeavesMissmatch",
      "msg": "NumberOfLeavesMissmatch"
    },
    {
      "code": 6008,
      "name": "MerkleTreePubkeysMissmatch",
      "msg": "MerkleTreePubkeysMissmatch"
    },
    {
      "code": 6009,
      "name": "NullifierArrayPubkeysMissmatch",
      "msg": "NullifierArrayPubkeysMissmatch"
    },
    {
      "code": 6010,
      "name": "InvalidNoopPubkey",
      "msg": "InvalidNoopPubkey"
    }
  ]
};