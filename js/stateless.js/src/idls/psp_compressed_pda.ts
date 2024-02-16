export type PspCompressedPda = {
  "version": "0.3.0",
  "name": "psp_compressed_pda",
  "instructions": [
    {
      "name": "executeCompressedTransaction",
      "docs": [
        "This function can be used to transfer sol and execute any other compressed transaction."
      ],
      "accounts": [
        {
          "name": "signer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authorityPda",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Check that mint authority is derived from signer"
          ]
        },
        {
          "name": "registeredProgramPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "CHECK this account"
          ]
        },
        {
          "name": "noopProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "CHECK this account"
          ]
        },
        {
          "name": "compressedPdaProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "pspAccountCompressionAuthority",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "CHECK this account in psp account compression program"
          ]
        },
        {
          "name": "accountCompressionProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "CHECK this account in psp account compression program"
          ]
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
            "name": "inUtxos",
            "type": {
              "vec": {
                "defined": "TokenInUtxo"
              }
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
            "name": "outUtxo",
            "type": {
              "vec": {
                "defined": "TokenOutUtxo"
              }
            }
          }
        ]
      }
    },
    {
      "name": "tokenInUtxo",
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
                "defined": "TlvData"
              }
            }
          }
        ]
      }
    },
    {
      "name": "tokenOutUtxo",
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
                "defined": "TlvData"
              }
            }
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "TlvData",
      "docs": [
        "Time lock escrow example:",
        "escrow tlv data -> compressed token program",
        "let escrow_data = {",
        "owner: Pubkey, // owner is the user pubkey",
        "release_slot: u64,",
        "deposit_slot: u64,",
        "};",
        "",
        "let escrow_tlv_data = TlvData {",
        "discriminator: [1,0,0,0,0,0,0,0],",
        "owner: escrow_program_id,",
        "data: escrow_data,",
        "tlv_data: Some(token_tlv.try_to_vec()?),",
        "};",
        "let token_tlv = TlvData {",
        "discriminator: [2,0,0,0,0,0,0,0],",
        "owner: token_program,",
        "data: token_data,",
        "tlv_data: None,",
        "};",
        "let token_data = TokenAccount {",
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
            "name": "tlvData",
            "type": {
              "option": {
                "defined": "Box"
              }
            }
          }
        ]
      }
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
        "This function can be used to transfer sol and execute any other compressed transaction."
      ],
      "accounts": [
        {
          "name": "signer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authorityPda",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Check that mint authority is derived from signer"
          ]
        },
        {
          "name": "registeredProgramPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "CHECK this account"
          ]
        },
        {
          "name": "noopProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "CHECK this account"
          ]
        },
        {
          "name": "compressedPdaProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "pspAccountCompressionAuthority",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "CHECK this account in psp account compression program"
          ]
        },
        {
          "name": "accountCompressionProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "CHECK this account in psp account compression program"
          ]
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
            "name": "inUtxos",
            "type": {
              "vec": {
                "defined": "TokenInUtxo"
              }
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
            "name": "outUtxo",
            "type": {
              "vec": {
                "defined": "TokenOutUtxo"
              }
            }
          }
        ]
      }
    },
    {
      "name": "tokenInUtxo",
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
                "defined": "TlvData"
              }
            }
          }
        ]
      }
    },
    {
      "name": "tokenOutUtxo",
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
                "defined": "TlvData"
              }
            }
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "TlvData",
      "docs": [
        "Time lock escrow example:",
        "escrow tlv data -> compressed token program",
        "let escrow_data = {",
        "owner: Pubkey, // owner is the user pubkey",
        "release_slot: u64,",
        "deposit_slot: u64,",
        "};",
        "",
        "let escrow_tlv_data = TlvData {",
        "discriminator: [1,0,0,0,0,0,0,0],",
        "owner: escrow_program_id,",
        "data: escrow_data,",
        "tlv_data: Some(token_tlv.try_to_vec()?),",
        "};",
        "let token_tlv = TlvData {",
        "discriminator: [2,0,0,0,0,0,0,0],",
        "owner: token_program,",
        "data: token_data,",
        "tlv_data: None,",
        "};",
        "let token_data = TokenAccount {",
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
            "name": "tlvData",
            "type": {
              "option": {
                "defined": "Box"
              }
            }
          }
        ]
      }
    }
  ]
};
