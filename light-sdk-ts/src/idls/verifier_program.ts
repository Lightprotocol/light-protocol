export type VerifierProgram = {
  version: "0.1.0";
  name: "verifier_program";
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
          name: "msg";
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
};

export const IDL: VerifierProgram = {
  version: "0.1.0",
  name: "verifier_program",
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
          name: "msg",
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
};
