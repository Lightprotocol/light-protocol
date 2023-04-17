export type VerifierProgramZero = {
  version: "0.1.0";
  name: "verifier_program_zero";
  instructions: [
    {
      name: "shieldedTransferInputs";
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
      name: "u256";
      type: {
        kind: "struct";
        fields: [
          {
            name: "x";
            type: "u64";
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
            name: "blinding";
            type: "u256";
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
            name: "transactionIndex";
            type: "u64";
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
      name: "shieldedTransferInputs",
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
      name: "u256",
      type: {
        kind: "struct",
        fields: [
          {
            name: "x",
            type: "u64",
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
            name: "blinding",
            type: "u256",
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
            name: "transactionIndex",
            type: "u64",
          },
        ],
      },
    },
  ],
};
