export type PspCompressedToken = {
  version: '0.3.0';
  name: 'psp_compressed_token';
  constants: [
    {
      name: 'PROGRAM_ID';
      type: 'string';
      value: '"9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE"';
    },
  ];
  instructions: [
    {
      name: 'createMint';
      docs: [
        'This instruction expects a mint account to be created in a separate token program instruction',
        'with token authority as mint authority.',
        'This instruction creates a token pool account for that mint owned by token authority.',
      ];
      accounts: [
        {
          name: 'feePayer';
          isMut: true;
          isSigner: true;
        },
        {
          name: 'authority';
          isMut: true;
          isSigner: true;
        },
        {
          name: 'tokenPoolPda';
          isMut: true;
          isSigner: false;
        },
        {
          name: 'systemProgram';
          isMut: false;
          isSigner: false;
        },
        {
          name: 'mint';
          isMut: true;
          isSigner: false;
        },
        {
          name: 'mintAuthorityPda';
          isMut: true;
          isSigner: false;
        },
        {
          name: 'tokenProgram';
          isMut: false;
          isSigner: false;
        },
      ];
      args: [];
    },
    {
      name: 'mintTo';
      accounts: [
        {
          name: 'feePayer';
          isMut: true;
          isSigner: true;
        },
        {
          name: 'authority';
          isMut: true;
          isSigner: true;
        },
        {
          name: 'mintAuthorityPda';
          isMut: true;
          isSigner: false;
        },
        {
          name: 'mint';
          isMut: true;
          isSigner: false;
        },
        {
          name: 'tokenPoolPda';
          isMut: true;
          isSigner: false;
        },
        {
          name: 'tokenProgram';
          isMut: false;
          isSigner: false;
        },
        {
          name: 'compressedPdaProgram';
          isMut: false;
          isSigner: false;
        },
        {
          name: 'registeredProgramPda';
          isMut: true;
          isSigner: false;
        },
        {
          name: 'noopProgram';
          isMut: false;
          isSigner: false;
        },
        {
          name: 'pspAccountCompressionAuthority';
          isMut: true;
          isSigner: false;
        },
        {
          name: 'accountCompressionProgram';
          isMut: false;
          isSigner: false;
        },
        {
          name: 'merkleTree';
          isMut: true;
          isSigner: false;
        },
      ];
      args: [
        {
          name: 'publicKeys';
          type: {
            vec: 'publicKey';
          };
        },
        {
          name: 'amounts';
          type: {
            vec: 'u64';
          };
        },
      ];
    },
    {
      name: 'transfer';
      accounts: [
        {
          name: 'feePayer';
          isMut: true;
          isSigner: true;
        },
        {
          name: 'authority';
          isMut: true;
          isSigner: true;
        },
        {
          name: 'cpiAuthorityPda';
          isMut: false;
          isSigner: false;
        },
        {
          name: 'compressedPdaProgram';
          isMut: false;
          isSigner: false;
        },
        {
          name: 'registeredProgramPda';
          isMut: true;
          isSigner: false;
        },
        {
          name: 'noopProgram';
          isMut: false;
          isSigner: false;
        },
        {
          name: 'pspAccountCompressionAuthority';
          isMut: true;
          isSigner: false;
        },
        {
          name: 'accountCompressionProgram';
          isMut: false;
          isSigner: false;
        },
        {
          name: 'selfProgram';
          isMut: false;
          isSigner: false;
        },
      ];
      args: [
        {
          name: 'inputs';
          type: 'bytes';
        },
      ];
    },
  ];
  accounts: [
    {
      name: 'InstructionDataTransferClient';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proof';
            type: {
              option: {
                defined: 'CompressedProofClient';
              };
            };
          },
          {
            name: 'rootIndices';
            type: {
              vec: 'u16';
            };
          },
          {
            name: 'inUtxos';
            type: {
              vec: {
                defined: 'InUtxoTupleClient';
              };
            };
          },
          {
            name: 'inTlvData';
            type: {
              vec: {
                defined: 'TokenTlvDataClient';
              };
            };
          },
          {
            name: 'outUtxos';
            type: {
              vec: {
                defined: 'TokenTransferOutUtxo';
              };
            };
          },
        ];
      };
    },
  ];
  types: [
    {
      name: 'PublicTransactionEvent';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'inUtxos';
            type: {
              vec: {
                defined: 'Utxo';
              };
            };
          },
          {
            name: 'outUtxos';
            type: {
              vec: {
                defined: 'Utxo';
              };
            };
          },
          {
            name: 'outUtxoIndices';
            type: {
              vec: 'u64';
            };
          },
          {
            name: 'deCompressAmount';
            type: {
              option: 'u64';
            };
          },
          {
            name: 'relayFee';
            type: {
              option: 'u64';
            };
          },
          {
            name: 'message';
            type: {
              option: 'bytes';
            };
          },
        ];
      };
    },
    {
      name: 'CpiSignature';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'program';
            type: 'publicKey';
          },
          {
            name: 'tlvHash';
            type: {
              array: ['u8', 32];
            };
          },
          {
            name: 'tlvData';
            type: {
              defined: 'TlvDataElement';
            };
          },
        ];
      };
    },
    {
      name: 'TlvSerializable';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'tlvElements';
            type: {
              vec: {
                defined: 'TlvDataElementSerializable';
              };
            };
          },
        ];
      };
    },
    {
      name: 'Tlv';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'tlvElements';
            type: {
              vec: {
                defined: 'TlvDataElement';
              };
            };
          },
        ];
      };
    },
    {
      name: 'TlvDataElementSerializable';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'discriminator';
            type: {
              array: ['u8', 8];
            };
          },
          {
            name: 'owner';
            type: 'u8';
          },
          {
            name: 'data';
            type: 'bytes';
          },
          {
            name: 'dataHash';
            type: {
              array: ['u8', 32];
            };
          },
        ];
      };
    },
    {
      name: 'TlvDataElement';
      docs: [
        'Time lock escrow example:',
        'escrow tlv data -> compressed token program',
        'let escrow_data = {',
        'owner: Pubkey, // owner is the user pubkey',
        'release_slot: u64,',
        'deposit_slot: u64,',
        '};',
        '',
        'let escrow_tlv_data = TlvDataElement {',
        'discriminator: [1,0,0,0,0,0,0,0],',
        'owner: escrow_program_id,',
        'data: escrow_data.try_to_vec()?,',
        '};',
        'let token_tlv = TlvDataElement {',
        'discriminator: [2,0,0,0,0,0,0,0],',
        'owner: token_program,',
        'data: token_data.try_to_vec()?,',
        '};',
        'let token_data = Account {',
        'mint,',
        'owner,',
        'amount: 10_000_000u64,',
        'delegate: None,',
        'state: Initialized, (u64)',
        'is_native: None,',
        'delegated_amount: 0u64,',
        'close_authority: None,',
        '};',
        '',
      ];
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'discriminator';
            type: {
              array: ['u8', 8];
            };
          },
          {
            name: 'owner';
            type: 'publicKey';
          },
          {
            name: 'data';
            type: 'bytes';
          },
          {
            name: 'dataHash';
            type: {
              array: ['u8', 32];
            };
          },
        ];
      };
    },
    {
      name: 'CompressedProof';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'a';
            type: {
              array: ['u8', 32];
            };
          },
          {
            name: 'b';
            type: {
              array: ['u8', 64];
            };
          },
          {
            name: 'c';
            type: {
              array: ['u8', 32];
            };
          },
        ];
      };
    },
    {
      name: 'InUtxoSerializableTuple';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'inUtxoSerializable';
            type: {
              defined: 'InUtxoSerializable';
            };
          },
          {
            name: 'indexMtAccount';
            type: 'u8';
          },
          {
            name: 'indexNullifierArrayAccount';
            type: 'u8';
          },
        ];
      };
    },
    {
      name: 'OutUtxoSerializableTuple';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'outUtxoSerializable';
            type: {
              defined: 'OutUtxoSerializable';
            };
          },
          {
            name: 'indexMtAccount';
            type: 'u8';
          },
        ];
      };
    },
    {
      name: 'InUtxoTuple';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'inUtxo';
            type: {
              defined: 'Utxo';
            };
          },
          {
            name: 'indexMtAccount';
            type: 'u8';
          },
          {
            name: 'indexNullifierArrayAccount';
            type: 'u8';
          },
        ];
      };
    },
    {
      name: 'OutUtxoTuple';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'outUtxo';
            type: {
              defined: 'OutUtxo';
            };
          },
          {
            name: 'indexMtAccount';
            type: 'u8';
          },
        ];
      };
    },
    {
      name: 'SerializedUtxos';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'pubkeyArray';
            type: {
              vec: 'publicKey';
            };
          },
          {
            name: 'u64Array';
            type: {
              vec: 'u64';
            };
          },
          {
            name: 'inUtxos';
            type: {
              vec: {
                defined: 'InUtxoSerializableTuple';
              };
            };
          },
          {
            name: 'outUtxos';
            type: {
              vec: {
                defined: 'OutUtxoSerializableTuple';
              };
            };
          },
        ];
      };
    },
    {
      name: 'InUtxoSerializable';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'owner';
            type: 'u8';
          },
          {
            name: 'leafIndex';
            type: 'u32';
          },
          {
            name: 'lamports';
            type: 'u8';
          },
          {
            name: 'data';
            type: {
              option: {
                defined: 'TlvSerializable';
              };
            };
          },
        ];
      };
    },
    {
      name: 'OutUtxoSerializable';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'owner';
            type: 'u8';
          },
          {
            name: 'lamports';
            type: 'u8';
          },
          {
            name: 'data';
            type: {
              option: {
                defined: 'TlvSerializable';
              };
            };
          },
        ];
      };
    },
    {
      name: 'OutUtxo';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'owner';
            type: 'publicKey';
          },
          {
            name: 'lamports';
            type: 'u64';
          },
          {
            name: 'data';
            type: {
              option: {
                defined: 'Tlv';
              };
            };
          },
        ];
      };
    },
    {
      name: 'Utxo';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'owner';
            type: 'publicKey';
          },
          {
            name: 'blinding';
            type: {
              array: ['u8', 32];
            };
          },
          {
            name: 'lamports';
            type: 'u64';
          },
          {
            name: 'data';
            type: {
              option: {
                defined: 'Tlv';
              };
            };
          },
        ];
      };
    },
    {
      name: 'UtxoClient';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'owner';
            type: 'publicKey';
          },
          {
            name: 'blinding';
            type: {
              array: ['u8', 32];
            };
          },
          {
            name: 'lamports';
            type: 'u64';
          },
          {
            name: 'data';
            type: {
              option: {
                defined: 'Tlv';
              };
            };
          },
        ];
      };
    },
    {
      name: 'InUtxoTupleClient';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'inUtxo';
            type: {
              defined: 'UtxoClient';
            };
          },
          {
            name: 'indexMtAccount';
            type: 'u8';
          },
          {
            name: 'indexNullifierArrayAccount';
            type: 'u8';
          },
        ];
      };
    },
    {
      name: 'CompressedProofClient';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'a';
            type: {
              array: ['u8', 32];
            };
          },
          {
            name: 'b';
            type: {
              array: ['u8', 64];
            };
          },
          {
            name: 'c';
            type: {
              array: ['u8', 32];
            };
          },
        ];
      };
    },
    {
      name: 'InstructionDataTransfer';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'proof';
            type: {
              option: {
                defined: 'CompressedProof';
              };
            };
          },
          {
            name: 'rootIndices';
            type: {
              vec: 'u16';
            };
          },
          {
            name: 'inUtxos';
            type: {
              vec: {
                defined: 'InUtxoTuple';
              };
            };
          },
          {
            name: 'inTlvData';
            type: {
              vec: {
                defined: 'TokenTlvData';
              };
            };
          },
          {
            name: 'outUtxos';
            type: {
              vec: {
                defined: 'TokenTransferOutUtxo';
              };
            };
          },
        ];
      };
    },
    {
      name: 'TokenTransferOutUtxo';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'owner';
            type: 'publicKey';
          },
          {
            name: 'amount';
            type: 'u64';
          },
          {
            name: 'lamports';
            type: {
              option: 'u64';
            };
          },
          {
            name: 'indexMtAccount';
            type: 'u8';
          },
        ];
      };
    },
    {
      name: 'TokenTlvData';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'mint';
            docs: ['The mint associated with this account'];
            type: 'publicKey';
          },
          {
            name: 'owner';
            docs: ['The owner of this account.'];
            type: 'publicKey';
          },
          {
            name: 'amount';
            docs: ['The amount of tokens this account holds.'];
            type: 'u64';
          },
          {
            name: 'delegate';
            docs: [
              'If `delegate` is `Some` then `delegated_amount` represents',
              'the amount authorized by the delegate',
            ];
            type: {
              option: 'publicKey';
            };
          },
          {
            name: 'state';
            docs: ["The account's state"];
            type: {
              defined: 'AccountState';
            };
          },
          {
            name: 'isNative';
            docs: [
              'If is_some, this is a native token, and the value logs the rent-exempt',
              'reserve. An Account is required to be rent-exempt, so the value is',
              'used by the Processor to ensure that wrapped SOL accounts do not',
              'drop below this threshold.',
            ];
            type: {
              option: 'u64';
            };
          },
          {
            name: 'delegatedAmount';
            docs: ['The amount delegated'];
            type: 'u64';
          },
        ];
      };
    },
    {
      name: 'TokenTlvDataClient';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'mint';
            docs: ['The mint associated with this account'];
            type: 'publicKey';
          },
          {
            name: 'owner';
            docs: ['The owner of this account.'];
            type: 'publicKey';
          },
          {
            name: 'amount';
            docs: ['The amount of tokens this account holds.'];
            type: 'u64';
          },
          {
            name: 'delegate';
            docs: [
              'If `delegate` is `Some` then `delegated_amount` represents',
              'the amount authorized by the delegate',
            ];
            type: {
              option: 'publicKey';
            };
          },
          {
            name: 'state';
            docs: ["The account's state"];
            type: 'u8';
          },
          {
            name: 'isNative';
            docs: [
              'If is_some, this is a native token, and the value logs the rent-exempt',
              'reserve. An Account is required to be rent-exempt, so the value is',
              'used by the Processor to ensure that wrapped SOL accounts do not',
              'drop below this threshold.',
            ];
            type: {
              option: 'u64';
            };
          },
          {
            name: 'delegatedAmount';
            docs: ['The amount delegated'];
            type: 'u64';
          },
        ];
      };
    },
    {
      name: 'AccountState';
      type: {
        kind: 'enum';
        variants: [
          {
            name: 'Uninitialized';
          },
          {
            name: 'Initialized';
          },
          {
            name: 'Frozen';
          },
        ];
      };
    },
  ];
  errors: [
    {
      code: 6000;
      name: 'PublicKeyAmountMissmatch';
      msg: 'public keys and amounts must be of same length';
    },
    {
      code: 6001;
      name: 'MissingNewAuthorityPda';
      msg: 'missing new authority pda';
    },
    {
      code: 6002;
      name: 'SignerCheckFailed';
      msg: 'SignerCheckFailed';
    },
    {
      code: 6003;
      name: 'MintCheckFailed';
      msg: 'MintCheckFailed';
    },
    {
      code: 6004;
      name: 'ComputeInputSumFailed';
      msg: 'ComputeInputSumFailed';
    },
    {
      code: 6005;
      name: 'ComputeOutputSumFailed';
      msg: 'ComputeOutputSumFailed';
    },
    {
      code: 6006;
      name: 'ComputeCompressSumFailed';
      msg: 'ComputeCompressSumFailed';
    },
    {
      code: 6007;
      name: 'ComputeDecompressSumFailed';
      msg: 'ComputeDecompressSumFailed';
    },
    {
      code: 6008;
      name: 'SumCheckFailed';
      msg: 'SumCheckFailed';
    },
  ];
};
export const IDL: PspCompressedToken = {
  version: '0.3.0',
  name: 'psp_compressed_token',
  constants: [
    {
      name: 'PROGRAM_ID',
      type: 'string',
      value: '"9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE"',
    },
  ],
  instructions: [
    {
      name: 'createMint',
      docs: [
        'This instruction expects a mint account to be created in a separate token program instruction',
        'with token authority as mint authority.',
        'This instruction creates a token pool account for that mint owned by token authority.',
      ],
      accounts: [
        {
          name: 'feePayer',
          isMut: true,
          isSigner: true,
        },
        {
          name: 'authority',
          isMut: true,
          isSigner: true,
        },
        {
          name: 'tokenPoolPda',
          isMut: true,
          isSigner: false,
        },
        {
          name: 'systemProgram',
          isMut: false,
          isSigner: false,
        },
        {
          name: 'mint',
          isMut: true,
          isSigner: false,
        },
        {
          name: 'mintAuthorityPda',
          isMut: true,
          isSigner: false,
        },
        {
          name: 'tokenProgram',
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: 'mintTo',
      accounts: [
        {
          name: 'feePayer',
          isMut: true,
          isSigner: true,
        },
        {
          name: 'authority',
          isMut: true,
          isSigner: true,
        },
        {
          name: 'mintAuthorityPda',
          isMut: true,
          isSigner: false,
        },
        {
          name: 'mint',
          isMut: true,
          isSigner: false,
        },
        {
          name: 'tokenPoolPda',
          isMut: true,
          isSigner: false,
        },
        {
          name: 'tokenProgram',
          isMut: false,
          isSigner: false,
        },
        {
          name: 'compressedPdaProgram',
          isMut: false,
          isSigner: false,
        },
        {
          name: 'registeredProgramPda',
          isMut: true,
          isSigner: false,
        },
        {
          name: 'noopProgram',
          isMut: false,
          isSigner: false,
        },
        {
          name: 'pspAccountCompressionAuthority',
          isMut: true,
          isSigner: false,
        },
        {
          name: 'accountCompressionProgram',
          isMut: false,
          isSigner: false,
        },
        {
          name: 'merkleTree',
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: 'publicKeys',
          type: {
            vec: 'publicKey',
          },
        },
        {
          name: 'amounts',
          type: {
            vec: 'u64',
          },
        },
      ],
    },
    {
      name: 'transfer',
      accounts: [
        {
          name: 'feePayer',
          isMut: true,
          isSigner: true,
        },
        {
          name: 'authority',
          isMut: true,
          isSigner: true,
        },
        {
          name: 'cpiAuthorityPda',
          isMut: false,
          isSigner: false,
        },
        {
          name: 'compressedPdaProgram',
          isMut: false,
          isSigner: false,
        },
        {
          name: 'registeredProgramPda',
          isMut: true,
          isSigner: false,
        },
        {
          name: 'noopProgram',
          isMut: false,
          isSigner: false,
        },
        {
          name: 'pspAccountCompressionAuthority',
          isMut: true,
          isSigner: false,
        },
        {
          name: 'accountCompressionProgram',
          isMut: false,
          isSigner: false,
        },
        {
          name: 'selfProgram',
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: 'inputs',
          type: 'bytes',
        },
      ],
    },
  ],
  accounts: [
    {
      name: 'InstructionDataTransferClient',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'proof',
            type: {
              option: {
                defined: 'CompressedProofClient',
              },
            },
          },
          {
            name: 'rootIndices',
            type: {
              vec: 'u16',
            },
          },
          {
            name: 'inUtxos',
            type: {
              vec: {
                defined: 'InUtxoTupleClient',
              },
            },
          },
          {
            name: 'inTlvData',
            type: {
              vec: {
                defined: 'TokenTlvDataClient',
              },
            },
          },
          {
            name: 'outUtxos',
            type: {
              vec: {
                defined: 'TokenTransferOutUtxo',
              },
            },
          },
        ],
      },
    },
  ],
  types: [
    {
      name: 'PublicTransactionEvent',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'inUtxos',
            type: {
              vec: {
                defined: 'Utxo',
              },
            },
          },
          {
            name: 'outUtxos',
            type: {
              vec: {
                defined: 'Utxo',
              },
            },
          },
          {
            name: 'outUtxoIndices',
            type: {
              vec: 'u64',
            },
          },
          {
            name: 'deCompressAmount',
            type: {
              option: 'u64',
            },
          },
          {
            name: 'relayFee',
            type: {
              option: 'u64',
            },
          },
          {
            name: 'message',
            type: {
              option: 'bytes',
            },
          },
        ],
      },
    },
    {
      name: 'CpiSignature',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'program',
            type: 'publicKey',
          },
          {
            name: 'tlvHash',
            type: {
              array: ['u8', 32],
            },
          },
          {
            name: 'tlvData',
            type: {
              defined: 'TlvDataElement',
            },
          },
        ],
      },
    },
    {
      name: 'TlvSerializable',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'tlvElements',
            type: {
              vec: {
                defined: 'TlvDataElementSerializable',
              },
            },
          },
        ],
      },
    },
    {
      name: 'Tlv',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'tlvElements',
            type: {
              vec: {
                defined: 'TlvDataElement',
              },
            },
          },
        ],
      },
    },
    {
      name: 'TlvDataElementSerializable',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'discriminator',
            type: {
              array: ['u8', 8],
            },
          },
          {
            name: 'owner',
            type: 'u8',
          },
          {
            name: 'data',
            type: 'bytes',
          },
          {
            name: 'dataHash',
            type: {
              array: ['u8', 32],
            },
          },
        ],
      },
    },
    {
      name: 'TlvDataElement',
      docs: [
        'Time lock escrow example:',
        'escrow tlv data -> compressed token program',
        'let escrow_data = {',
        'owner: Pubkey, // owner is the user pubkey',
        'release_slot: u64,',
        'deposit_slot: u64,',
        '};',
        '',
        'let escrow_tlv_data = TlvDataElement {',
        'discriminator: [1,0,0,0,0,0,0,0],',
        'owner: escrow_program_id,',
        'data: escrow_data.try_to_vec()?,',
        '};',
        'let token_tlv = TlvDataElement {',
        'discriminator: [2,0,0,0,0,0,0,0],',
        'owner: token_program,',
        'data: token_data.try_to_vec()?,',
        '};',
        'let token_data = Account {',
        'mint,',
        'owner,',
        'amount: 10_000_000u64,',
        'delegate: None,',
        'state: Initialized, (u64)',
        'is_native: None,',
        'delegated_amount: 0u64,',
        'close_authority: None,',
        '};',
        '',
      ],
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'discriminator',
            type: {
              array: ['u8', 8],
            },
          },
          {
            name: 'owner',
            type: 'publicKey',
          },
          {
            name: 'data',
            type: 'bytes',
          },
          {
            name: 'dataHash',
            type: {
              array: ['u8', 32],
            },
          },
        ],
      },
    },
    {
      name: 'CompressedProof',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'a',
            type: {
              array: ['u8', 32],
            },
          },
          {
            name: 'b',
            type: {
              array: ['u8', 64],
            },
          },
          {
            name: 'c',
            type: {
              array: ['u8', 32],
            },
          },
        ],
      },
    },
    {
      name: 'InUtxoSerializableTuple',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'inUtxoSerializable',
            type: {
              defined: 'InUtxoSerializable',
            },
          },
          {
            name: 'indexMtAccount',
            type: 'u8',
          },
          {
            name: 'indexNullifierArrayAccount',
            type: 'u8',
          },
        ],
      },
    },
    {
      name: 'OutUtxoSerializableTuple',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'outUtxoSerializable',
            type: {
              defined: 'OutUtxoSerializable',
            },
          },
          {
            name: 'indexMtAccount',
            type: 'u8',
          },
        ],
      },
    },
    {
      name: 'InUtxoTuple',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'inUtxo',
            type: {
              defined: 'Utxo',
            },
          },
          {
            name: 'indexMtAccount',
            type: 'u8',
          },
          {
            name: 'indexNullifierArrayAccount',
            type: 'u8',
          },
        ],
      },
    },
    {
      name: 'OutUtxoTuple',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'outUtxo',
            type: {
              defined: 'OutUtxo',
            },
          },
          {
            name: 'indexMtAccount',
            type: 'u8',
          },
        ],
      },
    },
    {
      name: 'SerializedUtxos',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'pubkeyArray',
            type: {
              vec: 'publicKey',
            },
          },
          {
            name: 'u64Array',
            type: {
              vec: 'u64',
            },
          },
          {
            name: 'inUtxos',
            type: {
              vec: {
                defined: 'InUtxoSerializableTuple',
              },
            },
          },
          {
            name: 'outUtxos',
            type: {
              vec: {
                defined: 'OutUtxoSerializableTuple',
              },
            },
          },
        ],
      },
    },
    {
      name: 'InUtxoSerializable',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'owner',
            type: 'u8',
          },
          {
            name: 'leafIndex',
            type: 'u32',
          },
          {
            name: 'lamports',
            type: 'u8',
          },
          {
            name: 'data',
            type: {
              option: {
                defined: 'TlvSerializable',
              },
            },
          },
        ],
      },
    },
    {
      name: 'OutUtxoSerializable',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'owner',
            type: 'u8',
          },
          {
            name: 'lamports',
            type: 'u8',
          },
          {
            name: 'data',
            type: {
              option: {
                defined: 'TlvSerializable',
              },
            },
          },
        ],
      },
    },
    {
      name: 'OutUtxo',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'owner',
            type: 'publicKey',
          },
          {
            name: 'lamports',
            type: 'u64',
          },
          {
            name: 'data',
            type: {
              option: {
                defined: 'Tlv',
              },
            },
          },
        ],
      },
    },
    {
      name: 'Utxo',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'owner',
            type: 'publicKey',
          },
          {
            name: 'blinding',
            type: {
              array: ['u8', 32],
            },
          },
          {
            name: 'lamports',
            type: 'u64',
          },
          {
            name: 'data',
            type: {
              option: {
                defined: 'Tlv',
              },
            },
          },
        ],
      },
    },
    {
      name: 'UtxoClient',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'owner',
            type: 'publicKey',
          },
          {
            name: 'blinding',
            type: {
              array: ['u8', 32],
            },
          },
          {
            name: 'lamports',
            type: 'u64',
          },
          {
            name: 'data',
            type: {
              option: {
                defined: 'Tlv',
              },
            },
          },
        ],
      },
    },
    {
      name: 'InUtxoTupleClient',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'inUtxo',
            type: {
              defined: 'UtxoClient',
            },
          },
          {
            name: 'indexMtAccount',
            type: 'u8',
          },
          {
            name: 'indexNullifierArrayAccount',
            type: 'u8',
          },
        ],
      },
    },
    {
      name: 'CompressedProofClient',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'a',
            type: {
              array: ['u8', 32],
            },
          },
          {
            name: 'b',
            type: {
              array: ['u8', 64],
            },
          },
          {
            name: 'c',
            type: {
              array: ['u8', 32],
            },
          },
        ],
      },
    },
    {
      name: 'InstructionDataTransfer',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'proof',
            type: {
              option: {
                defined: 'CompressedProof',
              },
            },
          },
          {
            name: 'rootIndices',
            type: {
              vec: 'u16',
            },
          },
          {
            name: 'inUtxos',
            type: {
              vec: {
                defined: 'InUtxoTuple',
              },
            },
          },
          {
            name: 'inTlvData',
            type: {
              vec: {
                defined: 'TokenTlvData',
              },
            },
          },
          {
            name: 'outUtxos',
            type: {
              vec: {
                defined: 'TokenTransferOutUtxo',
              },
            },
          },
        ],
      },
    },
    {
      name: 'TokenTransferOutUtxo',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'owner',
            type: 'publicKey',
          },
          {
            name: 'amount',
            type: 'u64',
          },
          {
            name: 'lamports',
            type: {
              option: 'u64',
            },
          },
          {
            name: 'indexMtAccount',
            type: 'u8',
          },
        ],
      },
    },
    {
      name: 'TokenTlvData',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'mint',
            docs: ['The mint associated with this account'],
            type: 'publicKey',
          },
          {
            name: 'owner',
            docs: ['The owner of this account.'],
            type: 'publicKey',
          },
          {
            name: 'amount',
            docs: ['The amount of tokens this account holds.'],
            type: 'u64',
          },
          {
            name: 'delegate',
            docs: [
              'If `delegate` is `Some` then `delegated_amount` represents',
              'the amount authorized by the delegate',
            ],
            type: {
              option: 'publicKey',
            },
          },
          {
            name: 'state',
            docs: ["The account's state"],
            type: {
              defined: 'AccountState',
            },
          },
          {
            name: 'isNative',
            docs: [
              'If is_some, this is a native token, and the value logs the rent-exempt',
              'reserve. An Account is required to be rent-exempt, so the value is',
              'used by the Processor to ensure that wrapped SOL accounts do not',
              'drop below this threshold.',
            ],
            type: {
              option: 'u64',
            },
          },
          {
            name: 'delegatedAmount',
            docs: ['The amount delegated'],
            type: 'u64',
          },
        ],
      },
    },
    {
      name: 'TokenTlvDataClient',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'mint',
            docs: ['The mint associated with this account'],
            type: 'publicKey',
          },
          {
            name: 'owner',
            docs: ['The owner of this account.'],
            type: 'publicKey',
          },
          {
            name: 'amount',
            docs: ['The amount of tokens this account holds.'],
            type: 'u64',
          },
          {
            name: 'delegate',
            docs: [
              'If `delegate` is `Some` then `delegated_amount` represents',
              'the amount authorized by the delegate',
            ],
            type: {
              option: 'publicKey',
            },
          },
          {
            name: 'state',
            docs: ["The account's state"],
            type: 'u8',
          },
          {
            name: 'isNative',
            docs: [
              'If is_some, this is a native token, and the value logs the rent-exempt',
              'reserve. An Account is required to be rent-exempt, so the value is',
              'used by the Processor to ensure that wrapped SOL accounts do not',
              'drop below this threshold.',
            ],
            type: {
              option: 'u64',
            },
          },
          {
            name: 'delegatedAmount',
            docs: ['The amount delegated'],
            type: 'u64',
          },
        ],
      },
    },
    {
      name: 'AccountState',
      type: {
        kind: 'enum',
        variants: [
          {
            name: 'Uninitialized',
          },
          {
            name: 'Initialized',
          },
          {
            name: 'Frozen',
          },
        ],
      },
    },
  ],
  errors: [
    {
      code: 6000,
      name: 'PublicKeyAmountMissmatch',
      msg: 'public keys and amounts must be of same length',
    },
    {
      code: 6001,
      name: 'MissingNewAuthorityPda',
      msg: 'missing new authority pda',
    },
    {
      code: 6002,
      name: 'SignerCheckFailed',
      msg: 'SignerCheckFailed',
    },
    {
      code: 6003,
      name: 'MintCheckFailed',
      msg: 'MintCheckFailed',
    },
    {
      code: 6004,
      name: 'ComputeInputSumFailed',
      msg: 'ComputeInputSumFailed',
    },
    {
      code: 6005,
      name: 'ComputeOutputSumFailed',
      msg: 'ComputeOutputSumFailed',
    },
    {
      code: 6006,
      name: 'ComputeCompressSumFailed',
      msg: 'ComputeCompressSumFailed',
    },
    {
      code: 6007,
      name: 'ComputeDecompressSumFailed',
      msg: 'ComputeDecompressSumFailed',
    },
    {
      code: 6008,
      name: 'SumCheckFailed',
      msg: 'SumCheckFailed',
    },
  ],
};
