export type VerifierProgramTwo = {
  version: "0.1.0";
  name: "verifier_program_two";
  constants: [
    {
      name: "PROGRAM_ID";
      type: "string";
      value: '"2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86"';
    },
  ];
  instructions: [
    {
      name: "shieldedTransferInputs";
      docs: [
        "This instruction is used to invoke this system verifier and can only be invoked via cpi.",
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
        {
          name: "verifierState";
          isMut: false;
          isSigner: true;
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
          name: "connectingHash";
          type: {
            array: ["u8", 32];
          };
        },
      ];
    },
  ];
  accounts: [
    {
      name: "zKtransactionApp4MainProofInputs";
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
              array: ["u8", 4];
            };
          },
          {
            name: "outputCommitment";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "publicAppVerifier";
            type: "u8";
          },
          {
            name: "transactionHash";
            type: "u8";
          },
          {
            name: "inAmount";
            type: {
              array: [
                {
                  array: ["u8", 2];
                },
                4,
              ];
            };
          },
          {
            name: "inPrivateKey";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "inBlinding";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "inAppDataHash";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "inPoolType";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "inVerifierPubkey";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "inPathIndices";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "inPathElements";
            type: {
              array: [
                {
                  array: ["u8", 18];
                },
                4,
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
                4,
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
                4,
              ];
            };
          },
          {
            name: "outPubkey";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "outBlinding";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "outAppDataHash";
            type: {
              array: ["u8", 4];
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
                4,
              ];
            };
          },
          {
            name: "outPoolType";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "outVerifierPubkey";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "assetPubkeys";
            type: {
              array: ["u8", 3];
            };
          },
          {
            name: "transactionVersion";
            type: "u8";
          },
          {
            name: "internalTxIntegrityHash";
            type: "u8";
          },
        ];
      };
    },
    {
      name: "zKtransactionApp4MainPublicInputs";
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
              array: ["u8", 4];
            };
          },
          {
            name: "outputCommitment";
            type: {
              array: ["u8", 4];
            };
          },
          {
            name: "publicAppVerifier";
            type: "u8";
          },
          {
            name: "transactionHash";
            type: "u8";
          },
        ];
      };
    },
  ];
  errors: [
    {
      code: 6000;
      name: "InvalidVerifier";
      msg: "System program is no valid verifier.";
    },
  ];
};

export const IDL: VerifierProgramTwo = {
  version: "0.1.0",
  name: "verifier_program_two",
  constants: [
    {
      name: "PROGRAM_ID",
      type: "string",
      value: '"2cxC8e8uNYLcymH6RTGuJs3N8fXGkwmMpw45pY65Ay86"',
    },
  ],
  instructions: [
    {
      name: "shieldedTransferInputs",
      docs: [
        "This instruction is used to invoke this system verifier and can only be invoked via cpi.",
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
        {
          name: "verifierState",
          isMut: false,
          isSigner: true,
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
          name: "connectingHash",
          type: {
            array: ["u8", 32],
          },
        },
      ],
    },
  ],
  accounts: [
    {
      name: "zKtransactionApp4MainProofInputs",
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
              array: ["u8", 4],
            },
          },
          {
            name: "outputCommitment",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "publicAppVerifier",
            type: "u8",
          },
          {
            name: "transactionHash",
            type: "u8",
          },
          {
            name: "inAmount",
            type: {
              array: [
                {
                  array: ["u8", 2],
                },
                4,
              ],
            },
          },
          {
            name: "inPrivateKey",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "inBlinding",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "inAppDataHash",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "inPoolType",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "inVerifierPubkey",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "inPathIndices",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "inPathElements",
            type: {
              array: [
                {
                  array: ["u8", 18],
                },
                4,
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
                4,
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
                4,
              ],
            },
          },
          {
            name: "outPubkey",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "outBlinding",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "outAppDataHash",
            type: {
              array: ["u8", 4],
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
                4,
              ],
            },
          },
          {
            name: "outPoolType",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "outVerifierPubkey",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "assetPubkeys",
            type: {
              array: ["u8", 3],
            },
          },
          {
            name: "transactionVersion",
            type: "u8",
          },
          {
            name: "internalTxIntegrityHash",
            type: "u8",
          },
        ],
      },
    },
    {
      name: "zKtransactionApp4MainPublicInputs",
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
              array: ["u8", 4],
            },
          },
          {
            name: "outputCommitment",
            type: {
              array: ["u8", 4],
            },
          },
          {
            name: "publicAppVerifier",
            type: "u8",
          },
          {
            name: "transactionHash",
            type: "u8",
          },
        ],
      },
    },
  ],
  errors: [
    {
      code: 6000,
      name: "InvalidVerifier",
      msg: "System program is no valid verifier.",
    },
  ],
};
