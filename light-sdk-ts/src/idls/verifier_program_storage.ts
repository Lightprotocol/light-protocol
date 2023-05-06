export type VerifierProgramStorage = {
  version: "0.1.0";
  name: "verifier_program_storage";
  instructions: [
    {
      name: "shieldedTransferFirst";
      docs: ["Saves the provided message in a temporary PDA."];
      accounts: [
        {
          name: "signingAddress";
          isMut: true;
          isSigner: true;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "verifierState";
          isMut: true;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "message";
          type: "bytes";
        },
      ];
    },
    {
      name: "shieldedTransferClose";
      docs: [
        "Close the temporary PDA. Should be used when we don't intend to perform",
        "the second transfer and want to reclaim the funds.",
      ];
      accounts: [
        {
          name: "signingAddress";
          isMut: true;
          isSigner: true;
        },
        {
          name: "verifierState";
          isMut: true;
          isSigner: false;
        },
      ];
      args: [];
    },
    {
      name: "shieldedTransferSecond";
      docs: [
        "Stores the provided message in a compressed account, closes the",
        "temporary PDA.",
      ];
      accounts: [
        {
          name: "signingAddress";
          isMut: true;
          isSigner: true;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "verifierState";
          isMut: true;
          isSigner: false;
        },
        {
          name: "programMerkleTree";
          isMut: false;
          isSigner: false;
        },
        {
          name: "logWrapper";
          isMut: false;
          isSigner: false;
          docs: ["CHECK"];
        },
        {
          name: "messageMerkleTree";
          isMut: true;
          isSigner: false;
        },
        {
          name: "transactionMerkleTree";
          isMut: true;
          isSigner: false;
        },
        {
          name: "authority";
          isMut: true;
          isSigner: false;
        },
        {
          name: "senderSol";
          isMut: true;
          isSigner: false;
        },
        {
          name: "recipientSol";
          isMut: true;
          isSigner: false;
        },
        {
          name: "relayerRecipientSol";
          isMut: true;
          isSigner: false;
        },
        {
          name: "registeredVerifierPda";
          isMut: true;
          isSigner: false;
          docs: ["Verifier config pda which needs to exist."];
        },
      ];
      args: [
        {
          name: "proofA";
          type: {
            array: ["u8", 64];
          };
        },
        {
          name: "proofB";
          type: {
            array: ["u8", 128];
          };
        },
        {
          name: "proofC";
          type: {
            array: ["u8", 64];
          };
        },
        {
          name: "nullifiers";
          type: {
            array: [
              {
                array: ["u8", 32];
              },
              2,
            ];
          };
        },
        {
          name: "leaves";
          type: {
            array: [
              {
                array: ["u8", 32];
              },
              2,
            ];
          };
        },
        {
          name: "publicAmountSol";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "rootIndex";
          type: "u64";
        },
        {
          name: "relayerFee";
          type: "u64";
        },
        {
          name: "encryptedUtxos";
          type: "bytes";
        },
      ];
    },
  ];
  accounts: [
    {
      name: "verifierState";
      type: {
        kind: "struct";
        fields: [
          {
            name: "msg";
            type: "bytes";
          },
        ];
      };
    },
    {
      name: "zKtransactionMasp2ProofInputs";
      type: {
        kind: "struct";
        fields: [
          {
            name: "root";
            type: "u8";
          },
          {
            name: "publicAmountSpl";
            type: "u8";
          },
          {
            name: "txIntegrityHash";
            type: "u8";
          },
          {
            name: "publicAmountSol";
            type: "u8";
          },
          {
            name: "publicMintPubkey";
            type: "u8";
          },
          {
            name: "inputNullifier";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "outputCommitment";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "inAmount";
            type: {
              array: [
                {
                  array: ["u8", 2];
                },
                2,
              ];
            };
          },
          {
            name: "inPrivateKey";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "inBlinding";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "inAppDataHash";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "inPathIndices";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "inPathElements";
            type: {
              array: [
                {
                  array: ["u8", 18];
                },
                2,
              ];
            };
          },
          {
            name: "inIndices";
            type: {
              array: [
                {
                  array: [
                    {
                      array: ["u8", 3];
                    },
                    2,
                  ];
                },
                2,
              ];
            };
          },
          {
            name: "outAmount";
            type: {
              array: [
                {
                  array: ["u8", 2];
                },
                2,
              ];
            };
          },
          {
            name: "outPubkey";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "outBlinding";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "outAppDataHash";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "outIndices";
            type: {
              array: [
                {
                  array: [
                    {
                      array: ["u8", 3];
                    },
                    2,
                  ];
                },
                2,
              ];
            };
          },
          {
            name: "outPoolType";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "outVerifierPubkey";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "inPoolType";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "inVerifierPubkey";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "transactionVersion";
            type: "u8";
          },
          {
            name: "assetPubkeys";
            type: {
              array: ["u8", 3];
            };
          },
          {
            name: "internalTxIntegrityHash";
            type: "u8";
          },
        ];
      };
    },
    {
      name: "zKtransactionMasp2PublicInputs";
      type: {
        kind: "struct";
        fields: [
          {
            name: "root";
            type: "u8";
          },
          {
            name: "publicAmountSpl";
            type: "u8";
          },
          {
            name: "txIntegrityHash";
            type: "u8";
          },
          {
            name: "publicAmountSol";
            type: "u8";
          },
          {
            name: "publicMintPubkey";
            type: "u8";
          },
          {
            name: "inputNullifier";
            type: {
              array: ["u8", 2];
            };
          },
          {
            name: "outputCommitment";
            type: {
              array: ["u8", 2];
            };
          },
        ];
      };
    },
  ];
  errors: [
    {
      code: 6000;
      name: "NoopProgram";
      msg: "The provided program is not the noop program.";
    },
    {
      code: 6001;
      name: "MessageTooLarge";
      msg: "Message too large, the limit per one method call is 1024 bytes.";
    },
    {
      code: 6002;
      name: "VerifierStateNoSpace";
      msg: "Cannot allocate more space for the verifier state account (message too large).";
    },
  ];
};

export const IDL: VerifierProgramStorage = {
  version: "0.1.0",
  name: "verifier_program_storage",
  instructions: [
    {
      name: "shieldedTransferFirst",
      docs: ["Saves the provided message in a temporary PDA."],
      accounts: [
        {
          name: "signingAddress",
          isMut: true,
          isSigner: true,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "verifierState",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "message",
          type: "bytes",
        },
      ],
    },
    {
      name: "shieldedTransferClose",
      docs: [
        "Close the temporary PDA. Should be used when we don't intend to perform",
        "the second transfer and want to reclaim the funds.",
      ],
      accounts: [
        {
          name: "signingAddress",
          isMut: true,
          isSigner: true,
        },
        {
          name: "verifierState",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "shieldedTransferSecond",
      docs: [
        "Stores the provided message in a compressed account, closes the",
        "temporary PDA.",
      ],
      accounts: [
        {
          name: "signingAddress",
          isMut: true,
          isSigner: true,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "verifierState",
          isMut: true,
          isSigner: false,
        },
        {
          name: "programMerkleTree",
          isMut: false,
          isSigner: false,
        },
        {
          name: "logWrapper",
          isMut: false,
          isSigner: false,
          docs: ["CHECK"],
        },
        {
          name: "messageMerkleTree",
          isMut: true,
          isSigner: false,
        },
        {
          name: "transactionMerkleTree",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: true,
          isSigner: false,
        },
        {
          name: "senderSol",
          isMut: true,
          isSigner: false,
        },
        {
          name: "recipientSol",
          isMut: true,
          isSigner: false,
        },
        {
          name: "relayerRecipientSol",
          isMut: true,
          isSigner: false,
        },
        {
          name: "registeredVerifierPda",
          isMut: true,
          isSigner: false,
          docs: ["Verifier config pda which needs to exist."],
        },
      ],
      args: [
        {
          name: "proofA",
          type: {
            array: ["u8", 64],
          },
        },
        {
          name: "proofB",
          type: {
            array: ["u8", 128],
          },
        },
        {
          name: "proofC",
          type: {
            array: ["u8", 64],
          },
        },
        {
          name: "nullifiers",
          type: {
            array: [
              {
                array: ["u8", 32],
              },
              2,
            ],
          },
        },
        {
          name: "leaves",
          type: {
            array: [
              {
                array: ["u8", 32],
              },
              2,
            ],
          },
        },
        {
          name: "publicAmountSol",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "rootIndex",
          type: "u64",
        },
        {
          name: "relayerFee",
          type: "u64",
        },
        {
          name: "encryptedUtxos",
          type: "bytes",
        },
      ],
    },
  ],
  accounts: [
    {
      name: "verifierState",
      type: {
        kind: "struct",
        fields: [
          {
            name: "msg",
            type: "bytes",
          },
        ],
      },
    },
    {
      name: "zKtransactionMasp2ProofInputs",
      type: {
        kind: "struct",
        fields: [
          {
            name: "root",
            type: "u8",
          },
          {
            name: "publicAmountSpl",
            type: "u8",
          },
          {
            name: "txIntegrityHash",
            type: "u8",
          },
          {
            name: "publicAmountSol",
            type: "u8",
          },
          {
            name: "publicMintPubkey",
            type: "u8",
          },
          {
            name: "inputNullifier",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "outputCommitment",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "inAmount",
            type: {
              array: [
                {
                  array: ["u8", 2],
                },
                2,
              ],
            },
          },
          {
            name: "inPrivateKey",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "inBlinding",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "inAppDataHash",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "inPathIndices",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "inPathElements",
            type: {
              array: [
                {
                  array: ["u8", 18],
                },
                2,
              ],
            },
          },
          {
            name: "inIndices",
            type: {
              array: [
                {
                  array: [
                    {
                      array: ["u8", 3],
                    },
                    2,
                  ],
                },
                2,
              ],
            },
          },
          {
            name: "outAmount",
            type: {
              array: [
                {
                  array: ["u8", 2],
                },
                2,
              ],
            },
          },
          {
            name: "outPubkey",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "outBlinding",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "outAppDataHash",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "outIndices",
            type: {
              array: [
                {
                  array: [
                    {
                      array: ["u8", 3],
                    },
                    2,
                  ],
                },
                2,
              ],
            },
          },
          {
            name: "outPoolType",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "outVerifierPubkey",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "inPoolType",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "inVerifierPubkey",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "transactionVersion",
            type: "u8",
          },
          {
            name: "assetPubkeys",
            type: {
              array: ["u8", 3],
            },
          },
          {
            name: "internalTxIntegrityHash",
            type: "u8",
          },
        ],
      },
    },
    {
      name: "zKtransactionMasp2PublicInputs",
      type: {
        kind: "struct",
        fields: [
          {
            name: "root",
            type: "u8",
          },
          {
            name: "publicAmountSpl",
            type: "u8",
          },
          {
            name: "txIntegrityHash",
            type: "u8",
          },
          {
            name: "publicAmountSol",
            type: "u8",
          },
          {
            name: "publicMintPubkey",
            type: "u8",
          },
          {
            name: "inputNullifier",
            type: {
              array: ["u8", 2],
            },
          },
          {
            name: "outputCommitment",
            type: {
              array: ["u8", 2],
            },
          },
        ],
      },
    },
  ],
  errors: [
    {
      code: 6000,
      name: "NoopProgram",
      msg: "The provided program is not the noop program.",
    },
    {
      code: 6001,
      name: "MessageTooLarge",
      msg: "Message too large, the limit per one method call is 1024 bytes.",
    },
    {
      code: 6002,
      name: "VerifierStateNoSpace",
      msg: "Cannot allocate more space for the verifier state account (message too large).",
    },
  ],
};
