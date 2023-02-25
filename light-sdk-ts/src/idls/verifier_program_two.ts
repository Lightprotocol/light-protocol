export type VerifierProgramTwo = {
  "version": "0.1.0",
  "name": "verifier_program_two",
  "instructions": [
    {
      "name": "shieldedTransferInputs",
      "docs": [
        "This instruction is used to invoke this system verifier and can only be invoked via cpi."
      ],
      "accounts": [
        {
          "name": "verifierState",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "programMerkleTree",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "sender",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipient",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "senderFee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientFee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "relayerRecipient",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Verifier config pda which needs ot exist Is not checked the relayer has complete freedom."
          ]
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": "bytes"
        },
        {
          "name": "connectingHash",
          "type": "bytes"
        }
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "InvalidVerifier",
      "msg": "System program is no valid verifier."
    }
  ]
};

export const IDL: VerifierProgramTwo = {
  "version": "0.1.0",
  "name": "verifier_program_two",
  "instructions": [
    {
      "name": "shieldedTransferInputs",
      "docs": [
        "This instruction is used to invoke this system verifier and can only be invoked via cpi."
      ],
      "accounts": [
        {
          "name": "verifierState",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "programMerkleTree",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "sender",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipient",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "senderFee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientFee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "relayerRecipient",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Verifier config pda which needs ot exist Is not checked the relayer has complete freedom."
          ]
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": "bytes"
        },
        {
          "name": "connectingHash",
          "type": "bytes"
        }
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "InvalidVerifier",
      "msg": "System program is no valid verifier."
    }
  ]
};
