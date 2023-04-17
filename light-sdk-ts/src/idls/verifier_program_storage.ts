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
