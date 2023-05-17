export type VerifierProgramZero = {
  version: "0.1.0";
  name: "verifier_program_zero";
  instructions: [
    {
      name: "shieldedTransferFirst";
      docs: [
        "This instruction is the first step of a shieled transaction.",
        "It creates and initializes a verifier state account to save state of a verification during",
        "computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data",
        "such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic",
        "in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2",
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
      name: "instructionDataShieldedTransferFirst";
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
                2,
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
      name: "u256";
      type: {
        kind: "struct";
        fields: [
          {
            name: "x";
            type: {
              array: ["u8", 32];
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
            name: "amounts";
            type: {
              array: ["u64", 2];
            };
          },
          {
            name: "splAssetIndex";
            type: "u64";
          },
          {
            name: "verifierAddressIndex";
            type: "u64";
          },
          {
            name: "blinding";
            type: "u256";
          },
          {
            name: "appDataHash";
            type: "u256";
          },
          {
            name: "accountShieldedPublicKey";
            type: "u256";
          },
          {
            name: "accountEncryptionPublicKey";
            type: {
              array: ["u8", 32];
            };
          },
        ];
      };
    },
    {
      name: "transactionParameters";
      type: {
        kind: "struct";
        fields: [
          {
            name: "message";
            type: "bytes";
          },
          {
            name: "inputUtxosBytes";
            type: {
              vec: "bytes";
            };
          },
          {
            name: "outputUtxosBytes";
            type: {
              vec: "bytes";
            };
          },
          {
            name: "recipientSpl";
            type: "publicKey";
          },
          {
            name: "recipientSol";
            type: "publicKey";
          },
          {
            name: "relayerPubkey";
            type: "publicKey";
          },
          {
            name: "relayerFee";
            type: "u64";
          },
          {
            name: "transactionNonce";
            type: "u64";
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
};

export const IDL: VerifierProgramZero = {
  version: "0.1.0",
  name: "verifier_program_zero",
  instructions: [
    {
      name: "shieldedTransferFirst",
      docs: [
        "This instruction is the first step of a shieled transaction.",
        "It creates and initializes a verifier state account to save state of a verification during",
        "computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data",
        "such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic",
        "in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2",
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
      name: "instructionDataShieldedTransferFirst",
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
                2,
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
      name: "u256",
      type: {
        kind: "struct",
        fields: [
          {
            name: "x",
            type: {
              array: ["u8", 32],
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
            name: "amounts",
            type: {
              array: ["u64", 2],
            },
          },
          {
            name: "splAssetIndex",
            type: "u64",
          },
          {
            name: "verifierAddressIndex",
            type: "u64",
          },
          {
            name: "blinding",
            type: "u256",
          },
          {
            name: "appDataHash",
            type: "u256",
          },
          {
            name: "accountShieldedPublicKey",
            type: "u256",
          },
          {
            name: "accountEncryptionPublicKey",
            type: {
              array: ["u8", 32],
            },
          },
        ],
      },
    },
    {
      name: "transactionParameters",
      type: {
        kind: "struct",
        fields: [
          {
            name: "message",
            type: "bytes",
          },
          {
            name: "inputUtxosBytes",
            type: {
              vec: "bytes",
            },
          },
          {
            name: "outputUtxosBytes",
            type: {
              vec: "bytes",
            },
          },
          {
            name: "recipientSpl",
            type: "publicKey",
          },
          {
            name: "recipientSol",
            type: "publicKey",
          },
          {
            name: "relayerPubkey",
            type: "publicKey",
          },
          {
            name: "relayerFee",
            type: "u64",
          },
          {
            name: "transactionNonce",
            type: "u64",
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
};
