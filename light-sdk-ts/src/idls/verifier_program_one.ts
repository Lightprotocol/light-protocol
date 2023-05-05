export type VerifierProgramOne = {
  version: "0.1.0";
  name: "verifier_program_one";
  constants: [
    {
      name: "EXECUTION_ORDER";
      type: {
        array: ["string", 2];
      };
      value: '["ShieldedTransferFirst" , "ShieldedTransferSecond"]';
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
            name: "nullifiers";
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
      name: "EXECUTION_ORDER",
      type: {
        array: ["string", 2],
      },
      value: '["ShieldedTransferFirst" , "ShieldedTransferSecond"]',
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
            name: "nullifiers",
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
