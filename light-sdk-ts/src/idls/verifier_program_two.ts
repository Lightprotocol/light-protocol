export type VerifierProgramTwo = {
  version: "0.1.0";
  name: "verifier_program_two";
  instructions: [
    {
      name: "shieldedTransferInputs";
      docs: [
        "This instruction is used to invoke this system verifier and can only be invoked via cpi.",
      ];
      accounts: [
        {
          name: "verifierState";
          isMut: false;
          isSigner: true;
        },
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
          name: "merkleTree";
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
          name: "connectingHash";
          type: {
            array: ["u8", 32];
          };
        },
      ];
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
  instructions: [
    {
      name: "shieldedTransferInputs",
      docs: [
        "This instruction is used to invoke this system verifier and can only be invoked via cpi.",
      ],
      accounts: [
        {
          name: "verifierState",
          isMut: false,
          isSigner: true,
        },
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
          name: "merkleTree",
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
          name: "connectingHash",
          type: {
            array: ["u8", 32],
          },
        },
      ],
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
