export type TestProver = {
  version: "0.1.0";
  name: "test_prover";
  instructions: [
    {
      name: "verifyProof";
      accounts: [];
      args: [
        {
          name: "publicInputs";
          type: {
            array: [
              {
                array: ["u8", 32];
              },
              1,
            ];
          };
        },
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
    },
  ];
  accounts: [
    {
      name: "zKposeidonProofInputs";
      type: {
        kind: "struct";
        fields: [
          {
            name: "hash";
            type: "u8";
          },
          {
            name: "x";
            type: "u8";
          },
        ];
      };
    },
    {
      name: "zKposeidonPublicInputs";
      type: {
        kind: "struct";
        fields: [
          {
            name: "hash";
            type: "u8";
          },
        ];
      };
    },
  ];
  errors: [
    {
      code: 6000;
      name: "ProofVerificationFailed";
      msg: "Proof verification failed.";
    },
  ];
};

export const IDL: TestProver = {
  version: "0.1.0",
  name: "test_prover",
  instructions: [
    {
      name: "verifyProof",
      accounts: [],
      args: [
        {
          name: "publicInputs",
          type: {
            array: [
              {
                array: ["u8", 32],
              },
              1,
            ],
          },
        },
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
  ],
  accounts: [
    {
      name: "zKposeidonProofInputs",
      type: {
        kind: "struct",
        fields: [
          {
            name: "hash",
            type: "u8",
          },
          {
            name: "x",
            type: "u8",
          },
        ],
      },
    },
    {
      name: "zKposeidonPublicInputs",
      type: {
        kind: "struct",
        fields: [
          {
            name: "hash",
            type: "u8",
          },
        ],
      },
    },
  ],
  errors: [
    {
      code: 6000,
      name: "ProofVerificationFailed",
      msg: "Proof verification failed.",
    },
  ],
};
