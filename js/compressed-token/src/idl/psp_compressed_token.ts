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
  types: [
    {
      name: 'CompressedAccountWithMerkleContext';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'compressedAccount';
            type: {
              defined: 'CompressedAccount';
            };
          },
          {
            name: 'indexMerkleTreeAccount';
            type: 'u8';
          },
          {
            name: 'indexNullifierArrayAccount';
            type: 'u8';
          },
          {
            name: 'leafIndex';
            type: 'u32';
          },
        ];
      };
    },
    {
      name: 'CompressedAccount';
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
            name: 'address';
            type: {
              option: {
                array: ['u8', 32];
              };
            };
          },
          {
            name: 'data';
            type: {
              option: {
                defined: 'CompressedAccountData';
              };
            };
          },
        ];
      };
    },
    {
      name: 'CompressedAccountData';
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
      name: 'PublicTransactionEvent';
      type: {
        kind: 'struct';
        fields: [
          {
            name: 'inputCompressedAccountHashes';
            type: {
              vec: {
                array: ['u8', 32];
              };
            };
          },
          {
            name: 'outputCompressedAccountHashes';
            type: {
              vec: {
                array: ['u8', 32];
              };
            };
          },
          {
            name: 'inputCompressedAccounts';
            type: {
              vec: {
                defined: 'CompressedAccountWithMerkleContext';
              };
            };
          },
          {
            name: 'outputCompressedAccounts';
            type: {
              vec: {
                defined: 'CompressedAccount';
              };
            };
          },
          {
            name: 'outputStateMerkleTreeAccountIndices';
            type: 'bytes';
          },
          {
            name: 'outputLeafIndices';
            type: {
              vec: 'u32';
            };
          },
          {
            name: 'relayFee';
            type: {
              option: 'u64';
            };
          },
          {
            name: 'deCompressAmount';
            type: {
              option: 'u64';
            };
          },
          {
            name: 'pubkeyArray';
            type: {
              vec: 'publicKey';
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
            name: 'inputRootIndices';
            type: {
              vec: 'u16';
            };
          },
          {
            name: 'newAddressSeeds';
            type: {
              vec: {
                array: ['u8', 32];
              };
            };
          },
          {
            name: 'addressQueueAccountIndices';
            type: 'bytes';
          },
          {
            name: 'addressMerkleTreeAccountIndices';
            type: 'bytes';
          },
          {
            name: 'addressMerkleTreeRootIndices';
            type: {
              vec: 'u16';
            };
          },
          {
            name: 'inputCompressedAccountsWithMerkleContext';
            type: {
              vec: {
                defined: 'CompressedAccountWithMerkleContext';
              };
            };
          },
          {
            name: 'outputCompressedAccounts';
            type: {
              vec: {
                defined: 'CompressedAccount';
              };
            };
          },
          {
            name: 'outputStateMerkleTreeAccountIndices';
            docs: [
              'The indices of the accounts in the output state merkle tree.',
            ];
            type: 'bytes';
          },
          {
            name: 'relayFee';
            type: {
              option: 'u64';
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
      name: 'CompressedTokenInstructionDataTransfer';
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
            name: 'inputCompressedAccountsWithMerkleContext';
            type: {
              vec: {
                defined: 'CompressedAccountWithMerkleContext';
              };
            };
          },
          {
            name: 'inputTokenData';
            type: {
              vec: {
                defined: 'TokenData';
              };
            };
          },
          {
            name: 'outputCompressedAccounts';
            type: {
              vec: {
                defined: 'TokenTransferOutputData';
              };
            };
          },
          {
            name: 'outputStateMerkleTreeAccountIndices';
            type: 'bytes';
          },
        ];
      };
    },
    {
      name: 'TokenTransferOutputData';
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
        ];
      };
    },
    {
      name: 'TokenData';
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
      name: 'TokenDataClient';
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
  types: [
    {
      name: 'CompressedAccountWithMerkleContext',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'compressedAccount',
            type: {
              defined: 'CompressedAccount',
            },
          },
          {
            name: 'indexMerkleTreeAccount',
            type: 'u8',
          },
          {
            name: 'indexNullifierArrayAccount',
            type: 'u8',
          },
          {
            name: 'leafIndex',
            type: 'u32',
          },
        ],
      },
    },
    {
      name: 'CompressedAccount',
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
            name: 'address',
            type: {
              option: {
                array: ['u8', 32],
              },
            },
          },
          {
            name: 'data',
            type: {
              option: {
                defined: 'CompressedAccountData',
              },
            },
          },
        ],
      },
    },
    {
      name: 'CompressedAccountData',
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
      name: 'PublicTransactionEvent',
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'inputCompressedAccountHashes',
            type: {
              vec: {
                array: ['u8', 32],
              },
            },
          },
          {
            name: 'outputCompressedAccountHashes',
            type: {
              vec: {
                array: ['u8', 32],
              },
            },
          },
          {
            name: 'inputCompressedAccounts',
            type: {
              vec: {
                defined: 'CompressedAccountWithMerkleContext',
              },
            },
          },
          {
            name: 'outputCompressedAccounts',
            type: {
              vec: {
                defined: 'CompressedAccount',
              },
            },
          },
          {
            name: 'outputStateMerkleTreeAccountIndices',
            type: 'bytes',
          },
          {
            name: 'outputLeafIndices',
            type: {
              vec: 'u32',
            },
          },
          {
            name: 'relayFee',
            type: {
              option: 'u64',
            },
          },
          {
            name: 'deCompressAmount',
            type: {
              option: 'u64',
            },
          },
          {
            name: 'pubkeyArray',
            type: {
              vec: 'publicKey',
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
            name: 'inputRootIndices',
            type: {
              vec: 'u16',
            },
          },
          {
            name: 'newAddressSeeds',
            type: {
              vec: {
                array: ['u8', 32],
              },
            },
          },
          {
            name: 'addressQueueAccountIndices',
            type: 'bytes',
          },
          {
            name: 'addressMerkleTreeAccountIndices',
            type: 'bytes',
          },
          {
            name: 'addressMerkleTreeRootIndices',
            type: {
              vec: 'u16',
            },
          },
          {
            name: 'inputCompressedAccountsWithMerkleContext',
            type: {
              vec: {
                defined: 'CompressedAccountWithMerkleContext',
              },
            },
          },
          {
            name: 'outputCompressedAccounts',
            type: {
              vec: {
                defined: 'CompressedAccount',
              },
            },
          },
          {
            name: 'outputStateMerkleTreeAccountIndices',
            docs: [
              'The indices of the accounts in the output state merkle tree.',
            ],
            type: 'bytes',
          },
          {
            name: 'relayFee',
            type: {
              option: 'u64',
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
      name: 'CompressedTokenInstructionDataTransfer',
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
            name: 'inputCompressedAccountsWithMerkleContext',
            type: {
              vec: {
                defined: 'CompressedAccountWithMerkleContext',
              },
            },
          },
          {
            name: 'inputTokenData',
            type: {
              vec: {
                defined: 'TokenData',
              },
            },
          },
          {
            name: 'outputCompressedAccounts',
            type: {
              vec: {
                defined: 'TokenTransferOutputData',
              },
            },
          },
          {
            name: 'outputStateMerkleTreeAccountIndices',
            type: 'bytes',
          },
        ],
      },
    },
    {
      name: 'TokenTransferOutputData',
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
        ],
      },
    },
    {
      name: 'TokenData',
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
      name: 'TokenDataClient',
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
