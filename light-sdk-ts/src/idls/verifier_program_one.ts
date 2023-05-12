export type VerifierProgramOne = {
  version: "0.1.0";
  name: "verifier_program_one";
  constants: [
    {
      name: "PROGRAM_ID";
      type: "string";
      value: '"3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL"';
    },
  ];
  instructions: [
    {
      name: "shieldedTransferFirst";
      docs: [
        "This instruction is the first step of a shielded transaction with 10 inputs and 2 outputs.",
        "It creates and initializes a verifier state account which stores public inputs and other data",
        "such as leaves, amounts, recipients, nullifiers, etc. to execute the verification and",
        "protocol logicin the second transaction.",
      ];
      accounts: [
        {
          name: "signingAddress";
          isMut: true;
          isSigner: true;
          docs: [
            "First transaction, therefore the signing address is not checked but saved to be checked in future instructions.",
          ];
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
          name: "inputs";
          type: "bytes";
        },
      ];
    },
    {
      name: "shieldedTransferSecond";
      docs: [
        "This instruction is the second step of a shieled transaction.",
        "The proof is verified with the parameters saved in the first transaction.",
        "At successful verification protocol logic is executed.",
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
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "programMerkleTree";
          isMut: false;
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
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "senderSpl";
          isMut: true;
          isSigner: false;
        },
        {
          name: "recipientSpl";
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
          name: "tokenAuthority";
          isMut: true;
          isSigner: false;
        },
        {
          name: "registeredVerifierPda";
          isMut: true;
          isSigner: false;
          docs: [
            "Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.",
          ];
        },
        {
          name: "logWrapper";
          isMut: false;
          isSigner: false;
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
      name: "closeVerifierState";
      docs: [
        "Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.",
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
  ];
  accounts: [
    {
      name: "zKtransactionMasp10PublicInputs";
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
              array: ["u8", 10];
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
    {
      name: "zKtransactionMasp10ProofInputs";
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
              array: ["u8", 10];
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
                10,
              ];
            };
          },
          {
            name: "inPrivateKey";
            type: {
              array: ["u8", 10];
            };
          },
          {
            name: "inBlinding";
            type: {
              array: ["u8", 10];
            };
          },
          {
            name: "inAppDataHash";
            type: {
              array: ["u8", 10];
            };
          },
          {
            name: "inPathIndices";
            type: {
              array: ["u8", 10];
            };
          },
          {
            name: "inPathElements";
            type: {
              array: [
                {
                  array: ["u8", 18];
                },
                10,
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
                10,
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
              array: ["u8", 10];
            };
          },
          {
            name: "inVerifierPubkey";
            type: {
              array: ["u8", 10];
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
      name: "instructionDataShieldedTransferFirst";
      type: {
        kind: "struct";
        fields: [
          {
            name: "publicAmountSpl";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "inputNullifier";
            type: {
              array: [
                {
                  array: ["u8", 32];
                },
                10,
              ];
            };
          },
          {
            name: "outputCommitment";
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
      };
    },
    {
      name: "instructionDataShieldedTransferSecond";
      type: {
        kind: "struct";
        fields: [
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
        ];
      };
    },
  ];
};

export const IDL: VerifierProgramOne = {
  version: "0.1.0",
  name: "verifier_program_one",
  constants: [
    {
      name: "PROGRAM_ID",
      type: "string",
      value: '"3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL"',
    },
  ],
  instructions: [
    {
      name: "shieldedTransferFirst",
      docs: [
        "This instruction is the first step of a shielded transaction with 10 inputs and 2 outputs.",
        "It creates and initializes a verifier state account which stores public inputs and other data",
        "such as leaves, amounts, recipients, nullifiers, etc. to execute the verification and",
        "protocol logicin the second transaction.",
      ],
      accounts: [
        {
          name: "signingAddress",
          isMut: true,
          isSigner: true,
          docs: [
            "First transaction, therefore the signing address is not checked but saved to be checked in future instructions.",
          ],
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
          name: "inputs",
          type: "bytes",
        },
      ],
    },
    {
      name: "shieldedTransferSecond",
      docs: [
        "This instruction is the second step of a shieled transaction.",
        "The proof is verified with the parameters saved in the first transaction.",
        "At successful verification protocol logic is executed.",
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
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "programMerkleTree",
          isMut: false,
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
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "senderSpl",
          isMut: true,
          isSigner: false,
        },
        {
          name: "recipientSpl",
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
          name: "tokenAuthority",
          isMut: true,
          isSigner: false,
        },
        {
          name: "registeredVerifierPda",
          isMut: true,
          isSigner: false,
          docs: [
            "Verifier config pda which needs ot exist Is not checked the relayer has complete freedom.",
          ],
        },
        {
          name: "logWrapper",
          isMut: false,
          isSigner: false,
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
      name: "closeVerifierState",
      docs: [
        "Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify.",
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
  ],
  accounts: [
    {
      name: "zKtransactionMasp10PublicInputs",
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
              array: ["u8", 10],
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
    {
      name: "zKtransactionMasp10ProofInputs",
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
              array: ["u8", 10],
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
                10,
              ],
            },
          },
          {
            name: "inPrivateKey",
            type: {
              array: ["u8", 10],
            },
          },
          {
            name: "inBlinding",
            type: {
              array: ["u8", 10],
            },
          },
          {
            name: "inAppDataHash",
            type: {
              array: ["u8", 10],
            },
          },
          {
            name: "inPathIndices",
            type: {
              array: ["u8", 10],
            },
          },
          {
            name: "inPathElements",
            type: {
              array: [
                {
                  array: ["u8", 18],
                },
                10,
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
                10,
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
              array: ["u8", 10],
            },
          },
          {
            name: "inVerifierPubkey",
            type: {
              array: ["u8", 10],
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
      name: "instructionDataShieldedTransferFirst",
      type: {
        kind: "struct",
        fields: [
          {
            name: "publicAmountSpl",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "inputNullifier",
            type: {
              array: [
                {
                  array: ["u8", 32],
                },
                10,
              ],
            },
          },
          {
            name: "outputCommitment",
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
    },
    {
      name: "instructionDataShieldedTransferSecond",
      type: {
        kind: "struct",
        fields: [
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
        ],
      },
    },
  ],
};
