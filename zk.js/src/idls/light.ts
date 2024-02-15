export type Light = {
  "version": "0.3.0",
  "name": "light",
  "constants": [
    {
      "name": "AUTHORITY_PDA_SEED",
      "type": "bytes",
      "value": "[97, 117, 116, 104, 111, 114, 105, 116, 121]"
    }
  ],
  "instructions": [
    {
      "name": "updateAuthority",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "groupPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "accountCompressionProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "bump",
          "type": "u8"
        },
        {
          "name": "newAuthority",
          "type": "publicKey"
        }
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "SumCheckFailed",
      "msg": "Sum check failed"
    }
  ]
};

export const IDL: Light = {
  "version": "0.3.0",
  "name": "light",
  "constants": [
    {
      "name": "AUTHORITY_PDA_SEED",
      "type": "bytes",
      "value": "[97, 117, 116, 104, 111, 114, 105, 116, 121]"
    }
  ],
  "instructions": [
    {
      "name": "updateAuthority",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "authorityPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "groupPda",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "accountCompressionProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "bump",
          "type": "u8"
        },
        {
          "name": "newAuthority",
          "type": "publicKey"
        }
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "SumCheckFailed",
      "msg": "Sum check failed"
    }
  ]
};
