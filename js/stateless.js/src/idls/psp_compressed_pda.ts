export type PspCompressedPda = {
  version: "0.3.0";
  name: "psp_compressed_pda";
  instructions: [
    {
      name: "executeCompressedTransaction";
      docs: [
        "This function can be used to transfer sol and execute any other compressed transaction.",
        "Instruction data is not optimized for space.",
        "This method can be called by cpi so that instruction data can be compressed with a custom algorithm.",
      ];
      accounts: [
        {
          name: "signer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "authorityPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "registeredProgramPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "noopProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "compressedPdaProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "pspAccountCompressionAuthority";
          isMut: true;
          isSigner: false;
        },
        {
          name: "accountCompressionProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "cpiSignatureAccount";
          isMut: false;
          isSigner: false;
          isOptional: true;
        },
      ];
      args: [
        {
          name: "inputs";
          type: "bytes";
        },
      ];
    },
    {
      name: "executeCompressedTransaction2";
      docs: [
        "This function can be used to transfer sol and execute any other compressed transaction.",
        "Instruction data is optimized for space.",
      ];
      accounts: [
        {
          name: "signer";
          isMut: true;
          isSigner: true;
        },
        {
          name: "authorityPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "registeredProgramPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "noopProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "compressedPdaProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "pspAccountCompressionAuthority";
          isMut: true;
          isSigner: false;
        },
        {
          name: "accountCompressionProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "cpiSignatureAccount";
          isMut: false;
          isSigner: false;
          isOptional: true;
        },
      ];
      args: [
        {
          name: "inputs";
          type: "bytes";
        },
      ];
    },
  ];
  accounts: [
    {
      name: "cpiSignatureAccount";
      type: {
        kind: "struct";
        fields: [
          {
            name: "signatures";
            type: {
              vec: {
                defined: "CpiSignature";
              };
            };
          },
        ];
      };
    },
    {
      name: "instructionDataTransfer";
      type: {
        kind: "struct";
        fields: [
          {
            name: "proofA";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "proofB";
            type: {
              array: ["u8", 64];
            };
          },
          {
            name: "proofC";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "lowElementIndices";
            type: {
              vec: "u16";
            };
          },
          {
            name: "rootIndices";
            type: {
              vec: "u64";
            };
          },
          {
            name: "rpcFee";
            type: {
              option: "u64";
            };
          },
          {
            name: "inUtxos";
            type: {
              vec: {
                defined: "Utxo";
              };
            };
          },
          {
            name: "outUtxos";
            type: {
              vec: {
                defined: "OutUtxo";
              };
            };
          },
          {
            name: "inUtxoMerkleTreeRemainingAccountIndex";
            type: "bytes";
          },
          {
            name: "inUtxoNullifierQueueRemainingAccountIndex";
            type: "bytes";
          },
          {
            name: "outUtxoMerkleTreeRemainingAccountIndex";
            type: "bytes";
          },
        ];
      };
    },
    {
      name: "instructionDataTransfer2";
      type: {
        kind: "struct";
        fields: [
          {
            name: "proofA";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "proofB";
            type: {
              array: ["u8", 64];
            };
          },
          {
            name: "proofC";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "lowElementIndices";
            type: {
              vec: "u16";
            };
          },
          {
            name: "rootIndices";
            type: {
              vec: "u64";
            };
          },
          {
            name: "rpcFee";
            type: {
              option: "u64";
            };
          },
          {
            name: "utxos";
            type: {
              defined: "SerializedUtxos";
            };
          },
          {
            name: "inUtxoMerkleTreeRemainingAccountIndex";
            type: "bytes";
          },
          {
            name: "inUtxoNullifierQueueRemainingAccountIndex";
            type: "bytes";
          },
          {
            name: "outUtxoMerkleTreeRemainingAccountIndex";
            type: "bytes";
          },
        ];
      };
    },
    {
      name: "inUtxoSerializable";
      type: {
        kind: "struct";
        fields: [
          {
            name: "owner";
            type: "u8";
          },
          {
            name: "leafIndex";
            type: "u32";
          },
          {
            name: "lamports";
            type: "u8";
          },
          {
            name: "data";
            type: {
              option: {
                defined: "TlvSerializable";
              };
            };
          },
        ];
      };
    },
    {
      name: "outUtxoSerializable";
      type: {
        kind: "struct";
        fields: [
          {
            name: "owner";
            type: "u8";
          },
          {
            name: "lamports";
            type: "u8";
          },
          {
            name: "data";
            type: {
              option: {
                defined: "TlvSerializable";
              };
            };
          },
        ];
      };
    },
    {
      name: "outUtxo";
      type: {
        kind: "struct";
        fields: [
          {
            name: "owner";
            type: "publicKey";
          },
          {
            name: "lamports";
            type: "u64";
          },
          {
            name: "data";
            type: {
              option: {
                defined: "Tlv";
              };
            };
          },
        ];
      };
    },
    {
      name: "utxo";
      type: {
        kind: "struct";
        fields: [
          {
            name: "owner";
            type: "publicKey";
          },
          {
            name: "blinding";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "lamports";
            type: "u64";
          },
          {
            name: "data";
            type: {
              option: {
                defined: "Tlv";
              };
            };
          },
        ];
      };
    },
  ];
  types: [
    {
      name: "CpiSignature";
      type: {
        kind: "struct";
        fields: [
          {
            name: "program";
            type: "publicKey";
          },
          {
            name: "tlvHash";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "tlvData";
            type: {
              defined: "TlvDataElement";
            };
          },
        ];
      };
    },
    {
      name: "SerializedUtxos";
      type: {
        kind: "struct";
        fields: [
          {
            name: "pubkeyArray";
            type: {
              vec: "publicKey";
            };
          },
          {
            name: "u64Array";
            type: {
              vec: "u64";
            };
          },
          {
            name: "inUtxos";
            type: {
              vec: {
                defined: "InUtxoSerializable";
              };
            };
          },
          {
            name: "outUtxos";
            type: {
              vec: {
                defined: "OutUtxoSerializable";
              };
            };
          },
        ];
      };
    },
    {
      name: "TlvSerializable";
      type: {
        kind: "struct";
        fields: [
          {
            name: "tlvElements";
            type: {
              vec: {
                defined: "TlvDataElementSerializable";
              };
            };
          },
        ];
      };
    },
    {
      name: "Tlv";
      type: {
        kind: "struct";
        fields: [
          {
            name: "tlvElements";
            type: {
              vec: {
                defined: "TlvDataElement";
              };
            };
          },
        ];
      };
    },
    {
      name: "TlvDataElementSerializable";
      type: {
        kind: "struct";
        fields: [
          {
            name: "discriminator";
            type: {
              array: ["u8", 8];
            };
          },
          {
            name: "owner";
            type: "u8";
          },
          {
            name: "data";
            type: "bytes";
          },
          {
            name: "dataHash";
            type: {
              array: ["u8", 32];
            };
          },
        ];
      };
    },
    {
      name: "TlvDataElement";
      docs: [
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
        "",
      ];
      type: {
        kind: "struct";
        fields: [
          {
            name: "discriminator";
            type: {
              array: ["u8", 8];
            };
          },
          {
            name: "owner";
            type: "publicKey";
          },
          {
            name: "data";
            type: "bytes";
          },
          {
            name: "dataHash";
            type: {
              array: ["u8", 32];
            };
          },
        ];
      };
    },
  ];
  errors: [
    {
      code: 6000;
      name: "SumCheckFailed";
      msg: "Sum check failed";
    },
  ];
};

export const IDL: PspCompressedPda = {
  version: "0.3.0",
  name: "psp_compressed_pda",
  instructions: [
    {
      name: "executeCompressedTransaction",
      docs: [
        "This function can be used to transfer sol and execute any other compressed transaction.",
        "Instruction data is not optimized for space.",
        "This method can be called by cpi so that instruction data can be compressed with a custom algorithm.",
      ],
      accounts: [
        {
          name: "signer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "authorityPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "registeredProgramPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "noopProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "compressedPdaProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "pspAccountCompressionAuthority",
          isMut: true,
          isSigner: false,
        },
        {
          name: "accountCompressionProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "cpiSignatureAccount",
          isMut: false,
          isSigner: false,
          isOptional: true,
        },
      ],
      args: [
        {
          name: "inputs",
          type: "bytes",
        },
      ],
    },
    {
      name: "executeCompressedTransaction2",
      docs: [
        "This function can be used to transfer sol and execute any other compressed transaction.",
        "Instruction data is optimized for space.",
      ],
      accounts: [
        {
          name: "signer",
          isMut: true,
          isSigner: true,
        },
        {
          name: "authorityPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "registeredProgramPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "noopProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "compressedPdaProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "pspAccountCompressionAuthority",
          isMut: true,
          isSigner: false,
        },
        {
          name: "accountCompressionProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "cpiSignatureAccount",
          isMut: false,
          isSigner: false,
          isOptional: true,
        },
      ],
      args: [
        {
          name: "inputs",
          type: "bytes",
        },
      ],
    },
  ],
  accounts: [
    {
      name: "cpiSignatureAccount",
      type: {
        kind: "struct",
        fields: [
          {
            name: "signatures",
            type: {
              vec: {
                defined: "CpiSignature",
              },
            },
          },
        ],
      },
    },
    {
      name: "instructionDataTransfer",
      type: {
        kind: "struct",
        fields: [
          {
            name: "proofA",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "proofB",
            type: {
              array: ["u8", 64],
            },
          },
          {
            name: "proofC",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "lowElementIndices",
            type: {
              vec: "u16",
            },
          },
          {
            name: "rootIndices",
            type: {
              vec: "u64",
            },
          },
          {
            name: "rpcFee",
            type: {
              option: "u64",
            },
          },
          {
            name: "inUtxos",
            type: {
              vec: {
                defined: "Utxo",
              },
            },
          },
          {
            name: "outUtxos",
            type: {
              vec: {
                defined: "OutUtxo",
              },
            },
          },
          {
            name: "inUtxoMerkleTreeRemainingAccountIndex",
            type: "bytes",
          },
          {
            name: "inUtxoNullifierQueueRemainingAccountIndex",
            type: "bytes",
          },
          {
            name: "outUtxoMerkleTreeRemainingAccountIndex",
            type: "bytes",
          },
        ],
      },
    },
    {
      name: "instructionDataTransfer2",
      type: {
        kind: "struct",
        fields: [
          {
            name: "proofA",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "proofB",
            type: {
              array: ["u8", 64],
            },
          },
          {
            name: "proofC",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "lowElementIndices",
            type: {
              vec: "u16",
            },
          },
          {
            name: "rootIndices",
            type: {
              vec: "u64",
            },
          },
          {
            name: "rpcFee",
            type: {
              option: "u64",
            },
          },
          {
            name: "utxos",
            type: {
              defined: "SerializedUtxos",
            },
          },
          {
            name: "inUtxoMerkleTreeRemainingAccountIndex",
            type: "bytes",
          },
          {
            name: "inUtxoNullifierQueueRemainingAccountIndex",
            type: "bytes",
          },
          {
            name: "outUtxoMerkleTreeRemainingAccountIndex",
            type: "bytes",
          },
        ],
      },
    },
    {
      name: "inUtxoSerializable",
      type: {
        kind: "struct",
        fields: [
          {
            name: "owner",
            type: "u8",
          },
          {
            name: "leafIndex",
            type: "u32",
          },
          {
            name: "lamports",
            type: "u8",
          },
          {
            name: "data",
            type: {
              option: {
                defined: "TlvSerializable",
              },
            },
          },
        ],
      },
    },
    {
      name: "outUtxoSerializable",
      type: {
        kind: "struct",
        fields: [
          {
            name: "owner",
            type: "u8",
          },
          {
            name: "lamports",
            type: "u8",
          },
          {
            name: "data",
            type: {
              option: {
                defined: "TlvSerializable",
              },
            },
          },
        ],
      },
    },
    {
      name: "outUtxo",
      type: {
        kind: "struct",
        fields: [
          {
            name: "owner",
            type: "publicKey",
          },
          {
            name: "lamports",
            type: "u64",
          },
          {
            name: "data",
            type: {
              option: {
                defined: "Tlv",
              },
            },
          },
        ],
      },
    },
    {
      name: "utxo",
      type: {
        kind: "struct",
        fields: [
          {
            name: "owner",
            type: "publicKey",
          },
          {
            name: "blinding",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "lamports",
            type: "u64",
          },
          {
            name: "data",
            type: {
              option: {
                defined: "Tlv",
              },
            },
          },
        ],
      },
    },
  ],
  types: [
    {
      name: "CpiSignature",
      type: {
        kind: "struct",
        fields: [
          {
            name: "program",
            type: "publicKey",
          },
          {
            name: "tlvHash",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "tlvData",
            type: {
              defined: "TlvDataElement",
            },
          },
        ],
      },
    },
    {
      name: "SerializedUtxos",
      type: {
        kind: "struct",
        fields: [
          {
            name: "pubkeyArray",
            type: {
              vec: "publicKey",
            },
          },
          {
            name: "u64Array",
            type: {
              vec: "u64",
            },
          },
          {
            name: "inUtxos",
            type: {
              vec: {
                defined: "InUtxoSerializable",
              },
            },
          },
          {
            name: "outUtxos",
            type: {
              vec: {
                defined: "OutUtxoSerializable",
              },
            },
          },
        ],
      },
    },
    {
      name: "TlvSerializable",
      type: {
        kind: "struct",
        fields: [
          {
            name: "tlvElements",
            type: {
              vec: {
                defined: "TlvDataElementSerializable",
              },
            },
          },
        ],
      },
    },
    {
      name: "Tlv",
      type: {
        kind: "struct",
        fields: [
          {
            name: "tlvElements",
            type: {
              vec: {
                defined: "TlvDataElement",
              },
            },
          },
        ],
      },
    },
    {
      name: "TlvDataElementSerializable",
      type: {
        kind: "struct",
        fields: [
          {
            name: "discriminator",
            type: {
              array: ["u8", 8],
            },
          },
          {
            name: "owner",
            type: "u8",
          },
          {
            name: "data",
            type: "bytes",
          },
          {
            name: "dataHash",
            type: {
              array: ["u8", 32],
            },
          },
        ],
      },
    },
    {
      name: "TlvDataElement",
      docs: [
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
        "",
      ],
      type: {
        kind: "struct",
        fields: [
          {
            name: "discriminator",
            type: {
              array: ["u8", 8],
            },
          },
          {
            name: "owner",
            type: "publicKey",
          },
          {
            name: "data",
            type: "bytes",
          },
          {
            name: "dataHash",
            type: {
              array: ["u8", 32],
            },
          },
        ],
      },
    },
  ],
  errors: [
    {
      code: 6000,
      name: "SumCheckFailed",
      msg: "Sum check failed",
    },
  ],
};
