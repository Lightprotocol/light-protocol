export type LightPsp2in2outStorage = {
  "version": "0.3.1",
  "name": "light_psp2in2out_storage",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj\""
    },
    {
      "name": "MESSAGE_PER_CALL_SIZE",
      "type": {
        "defined": "usize"
      },
      "value": "1024"
    },
    {
      "name": "MESSAGE_MAX_SIZE",
      "type": {
        "defined": "usize"
      },
      "value": "2048"
    },
    {
      "name": "VERIFIER_STATE_MAX_SIZE",
      "type": {
        "defined": "usize"
      },
      "value": "MESSAGE_MAX_SIZE + 8"
    },
    {
      "name": "ENCRYPTED_UTXOS_SIZE",
      "type": {
        "defined": "usize"
      },
      "value": "256"
    },
    {
      "name": "VERIFYINGKEY_TRANSACTION_MASP2_MAIN",
      "type": {
        "defined": "Groth16Verifyingkey"
      },
      "value": "Groth16Verifyingkey { nr_pubinputs : 9 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [26 , 183 , 224 , 209 , 179 , 32 , 134 , 249 , 52 , 226 , 164 , 203 , 57 , 100 , 108 , 149 , 128 , 234 , 197 , 76 , 218 , 197 , 25 , 91 , 243 , 82 , 102 , 236 , 15 , 236 , 129 , 223 , 30 , 74 , 228 , 61 , 51 , 0 , 117 , 160 , 231 , 189 , 129 , 148 , 81 , 49 , 100 , 8 , 91 , 86 , 117 , 34 , 80 , 156 , 28 , 102 , 133 , 183 , 30 , 119 , 34 , 242 , 102 , 85 , 38 , 170 , 40 , 49 , 234 , 20 , 79 , 204 , 114 , 137 , 154 , 243 , 4 , 227 , 123 , 139 , 102 , 61 , 60 , 64 , 200 , 4 , 65 , 47 , 162 , 251 , 29 , 0 , 186 , 201 , 73 , 216 , 41 , 8 , 46 , 88 , 153 , 31 , 200 , 173 , 115 , 222 , 192 , 183 , 26 , 10 , 210 , 59 , 166 , 89 , 8 , 119 , 4 , 95 , 71 , 32 , 59 , 212 , 206 , 150 , 144 , 5 , 90 , 93] , vk_ic : & [[13 , 47 , 65 , 180 , 19 , 79 , 13 , 73 , 25 , 237 , 213 , 84 , 210 , 122 , 59 , 197 , 215 , 137 , 56 , 107 , 184 , 200 , 76 , 85 , 254 , 107 , 43 , 102 , 69 , 161 , 86 , 82 , 37 , 230 , 159 , 121 , 92 , 241 , 164 , 116 , 95 , 61 , 48 , 117 , 88 , 138 , 147 , 214 , 231 , 236 , 119 , 48 , 11 , 175 , 207 , 98 , 113 , 39 , 116 , 99 , 137 , 144 , 31 , 172] , [3 , 186 , 25 , 234 , 219 , 45 , 95 , 147 , 5 , 176 , 144 , 130 , 5 , 72 , 3 , 146 , 178 , 110 , 75 , 238 , 42 , 90 , 76 , 12 , 80 , 73 , 41 , 197 , 224 , 6 , 14 , 200 , 44 , 234 , 188 , 156 , 18 , 126 , 183 , 171 , 172 , 198 , 152 , 235 , 36 , 151 , 214 , 43 , 211 , 19 , 130 , 83 , 4 , 0 , 90 , 22 , 224 , 253 , 44 , 198 , 211 , 227 , 223 , 30] , [28 , 1 , 93 , 168 , 117 , 41 , 164 , 93 , 250 , 136 , 67 , 122 , 245 , 117 , 141 , 119 , 68 , 42 , 52 , 135 , 25 , 26 , 237 , 134 , 97 , 149 , 23 , 61 , 29 , 228 , 201 , 151 , 45 , 85 , 215 , 65 , 201 , 160 , 189 , 240 , 1 , 213 , 80 , 86 , 79 , 198 , 239 , 207 , 102 , 25 , 236 , 53 , 19 , 15 , 252 , 246 , 81 , 237 , 169 , 42 , 219 , 18 , 32 , 52] , [9 , 50 , 43 , 223 , 76 , 108 , 146 , 164 , 3 , 6 , 118 , 136 , 69 , 180 , 17 , 33 , 50 , 66 , 253 , 163 , 222 , 239 , 29 , 4 , 0 , 9 , 143 , 162 , 157 , 196 , 42 , 10 , 37 , 160 , 245 , 101 , 212 , 223 , 100 , 9 , 183 , 244 , 141 , 151 , 87 , 241 , 240 , 244 , 109 , 90 , 24 , 95 , 204 , 109 , 255 , 40 , 22 , 151 , 100 , 215 , 17 , 221 , 203 , 113] , [34 , 101 , 54 , 1 , 33 , 144 , 72 , 18 , 180 , 136 , 29 , 111 , 253 , 3 , 75 , 166 , 117 , 111 , 33 , 39 , 161 , 203 , 238 , 87 , 171 , 140 , 220 , 46 , 158 , 66 , 232 , 5 , 21 , 199 , 117 , 43 , 29 , 67 , 10 , 244 , 185 , 13 , 71 , 253 , 204 , 10 , 120 , 152 , 254 , 50 , 178 , 39 , 105 , 136 , 110 , 237 , 67 , 210 , 116 , 15 , 70 , 154 , 135 , 156] , [17 , 215 , 5 , 205 , 156 , 89 , 128 , 154 , 152 , 12 , 106 , 140 , 217 , 196 , 33 , 97 , 212 , 218 , 47 , 80 , 220 , 77 , 35 , 116 , 27 , 32 , 183 , 115 , 63 , 189 , 230 , 79 , 40 , 237 , 49 , 85 , 37 , 20 , 186 , 128 , 94 , 222 , 99 , 2 , 4 , 105 , 77 , 202 , 220 , 83 , 80 , 245 , 26 , 107 , 75 , 242 , 47 , 139 , 157 , 175 , 18 , 205 , 169 , 8] , [42 , 117 , 183 , 73 , 98 , 148 , 197 , 201 , 93 , 12 , 53 , 246 , 46 , 186 , 70 , 201 , 56 , 209 , 160 , 45 , 221 , 68 , 174 , 178 , 215 , 39 , 112 , 157 , 152 , 240 , 63 , 79 , 6 , 204 , 150 , 68 , 13 , 234 , 80 , 127 , 138 , 99 , 215 , 171 , 200 , 175 , 207 , 39 , 7 , 251 , 216 , 82 , 146 , 161 , 201 , 114 , 21 , 251 , 170 , 107 , 219 , 125 , 35 , 134] , [44 , 201 , 91 , 12 , 124 , 52 , 89 , 66 , 238 , 169 , 182 , 108 , 234 , 249 , 77 , 145 , 69 , 255 , 125 , 162 , 194 , 122 , 77 , 160 , 183 , 24 , 179 , 199 , 84 , 254 , 176 , 5 , 4 , 193 , 184 , 38 , 123 , 166 , 250 , 227 , 66 , 43 , 93 , 186 , 207 , 209 , 193 , 55 , 241 , 180 , 124 , 195 , 21 , 85 , 13 , 59 , 99 , 107 , 200 , 178 , 187 , 164 , 119 , 81] , [30 , 32 , 165 , 174 , 31 , 92 , 79 , 57 , 27 , 163 , 232 , 92 , 134 , 193 , 67 , 122 , 95 , 69 , 215 , 49 , 196 , 69 , 156 , 94 , 95 , 28 , 186 , 93 , 159 , 9 , 73 , 5 , 35 , 152 , 68 , 60 , 113 , 21 , 11 , 114 , 19 , 53 , 172 , 118 , 6 , 7 , 31 , 145 , 154 , 133 , 85 , 76 , 32 , 247 , 213 , 242 , 174 , 237 , 9 , 39 , 202 , 202 , 118 , 172] , [35 , 215 , 169 , 66 , 125 , 152 , 99 , 8 , 208 , 232 , 15 , 28 , 237 , 68 , 188 , 64 , 54 , 89 , 195 , 239 , 193 , 225 , 53 , 7 , 40 , 182 , 162 , 142 , 245 , 199 , 193 , 131 , 9 , 27 , 236 , 217 , 147 , 18 , 21 , 145 , 251 , 117 , 146 , 126 , 207 , 128 , 12 , 75 , 65 , 144 , 242 , 213 , 106 , 45 , 226 , 82 , 18 , 167 , 38 , 27 , 176 , 217 , 33 , 161]] , }"
    }
  ],
  "instructions": [
    {
      "name": "shieldedTransferFirst",
      "docs": [
        "Saves the provided message in a temporary PDA."
      ],
      "accounts": [
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
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "shieldedTransferClose",
      "docs": [
        "Close the temporary PDA. Should be used when we don't intend to perform",
        "the second transfer and want to reclaim the funds."
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "shieldedTransferSecond",
      "docs": [
        "Stores the provided message in a compressed account, closes the",
        "temporary PDA."
      ],
      "accounts": [
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
          "name": "transactionMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "rpcRecipientSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "senderSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Verifier config pda which needs to exist."
          ]
        },
        {
          "name": "logWrapper",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "eventMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "verifierState",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "msg",
            "type": "bytes"
          }
        ]
      }
    },
    {
      "name": "instructionDataShieldedTransferFirst",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "message",
            "type": "bytes"
          }
        ]
      }
    },
    {
      "name": "instructionDataShieldedTransferSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proofA",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofB",
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          },
          {
            "name": "proofC",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "inputNullifier",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "publicAmountSol",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "rootIndex",
            "type": "u64"
          },
          {
            "name": "rpcFee",
            "type": "u64"
          },
          {
            "name": "encryptedUtxos",
            "type": {
              "array": [
                "u8",
                256
              ]
            }
          }
        ]
      }
    },
    {
      "name": "zKtransactionMasp2MainProofInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "root",
            "type": "u8"
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "txIntegrityHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPubkey",
            "type": "u8"
          },
          {
            "name": "inputNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "inPrivateKey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inBlinding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inAppDataHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inPathIndices",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inPathElements",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    18
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "inIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "outAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "outPubkey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outBlinding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outAppDataHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "outPoolType",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outVerifierPubkey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inPoolType",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inVerifierPubkey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "transactionVersion",
            "type": "u8"
          },
          {
            "name": "assetPubkeys",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "internalTxIntegrityHash",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "zKtransactionMasp2MainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "root",
            "type": "u8"
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "txIntegrityHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPubkey",
            "type": "u8"
          },
          {
            "name": "inputNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          }
        ]
      }
    },
    {
      "name": "instructionDataLightInstructionTransactionMasp2MainSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "root",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicAmountSpl",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "txIntegrityHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicAmountSol",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicMintPubkey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "inputNullifier",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                2
              ]
            }
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "NoopProgram",
      "msg": "The provided program is not the noop program."
    },
    {
      "code": 6001,
      "name": "MessageTooLarge",
      "msg": "Message too large, the limit per one method call is 1024 bytes."
    },
    {
      "code": 6002,
      "name": "VerifierStateNoSpace",
      "msg": "Cannot allocate more space for the verifier state account (message too large)."
    }
  ]
};

export const IDL: LightPsp2in2outStorage = {
  "version": "0.3.1",
  "name": "light_psp2in2out_storage",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"DJpbogMSrK94E1zvvJydtkqoE4sknuzmMRoutd6B7TKj\""
    },
    {
      "name": "MESSAGE_PER_CALL_SIZE",
      "type": {
        "defined": "usize"
      },
      "value": "1024"
    },
    {
      "name": "MESSAGE_MAX_SIZE",
      "type": {
        "defined": "usize"
      },
      "value": "2048"
    },
    {
      "name": "VERIFIER_STATE_MAX_SIZE",
      "type": {
        "defined": "usize"
      },
      "value": "MESSAGE_MAX_SIZE + 8"
    },
    {
      "name": "ENCRYPTED_UTXOS_SIZE",
      "type": {
        "defined": "usize"
      },
      "value": "256"
    },
    {
      "name": "VERIFYINGKEY_TRANSACTION_MASP2_MAIN",
      "type": {
        "defined": "Groth16Verifyingkey"
      },
      "value": "Groth16Verifyingkey { nr_pubinputs : 9 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [26 , 183 , 224 , 209 , 179 , 32 , 134 , 249 , 52 , 226 , 164 , 203 , 57 , 100 , 108 , 149 , 128 , 234 , 197 , 76 , 218 , 197 , 25 , 91 , 243 , 82 , 102 , 236 , 15 , 236 , 129 , 223 , 30 , 74 , 228 , 61 , 51 , 0 , 117 , 160 , 231 , 189 , 129 , 148 , 81 , 49 , 100 , 8 , 91 , 86 , 117 , 34 , 80 , 156 , 28 , 102 , 133 , 183 , 30 , 119 , 34 , 242 , 102 , 85 , 38 , 170 , 40 , 49 , 234 , 20 , 79 , 204 , 114 , 137 , 154 , 243 , 4 , 227 , 123 , 139 , 102 , 61 , 60 , 64 , 200 , 4 , 65 , 47 , 162 , 251 , 29 , 0 , 186 , 201 , 73 , 216 , 41 , 8 , 46 , 88 , 153 , 31 , 200 , 173 , 115 , 222 , 192 , 183 , 26 , 10 , 210 , 59 , 166 , 89 , 8 , 119 , 4 , 95 , 71 , 32 , 59 , 212 , 206 , 150 , 144 , 5 , 90 , 93] , vk_ic : & [[13 , 47 , 65 , 180 , 19 , 79 , 13 , 73 , 25 , 237 , 213 , 84 , 210 , 122 , 59 , 197 , 215 , 137 , 56 , 107 , 184 , 200 , 76 , 85 , 254 , 107 , 43 , 102 , 69 , 161 , 86 , 82 , 37 , 230 , 159 , 121 , 92 , 241 , 164 , 116 , 95 , 61 , 48 , 117 , 88 , 138 , 147 , 214 , 231 , 236 , 119 , 48 , 11 , 175 , 207 , 98 , 113 , 39 , 116 , 99 , 137 , 144 , 31 , 172] , [3 , 186 , 25 , 234 , 219 , 45 , 95 , 147 , 5 , 176 , 144 , 130 , 5 , 72 , 3 , 146 , 178 , 110 , 75 , 238 , 42 , 90 , 76 , 12 , 80 , 73 , 41 , 197 , 224 , 6 , 14 , 200 , 44 , 234 , 188 , 156 , 18 , 126 , 183 , 171 , 172 , 198 , 152 , 235 , 36 , 151 , 214 , 43 , 211 , 19 , 130 , 83 , 4 , 0 , 90 , 22 , 224 , 253 , 44 , 198 , 211 , 227 , 223 , 30] , [28 , 1 , 93 , 168 , 117 , 41 , 164 , 93 , 250 , 136 , 67 , 122 , 245 , 117 , 141 , 119 , 68 , 42 , 52 , 135 , 25 , 26 , 237 , 134 , 97 , 149 , 23 , 61 , 29 , 228 , 201 , 151 , 45 , 85 , 215 , 65 , 201 , 160 , 189 , 240 , 1 , 213 , 80 , 86 , 79 , 198 , 239 , 207 , 102 , 25 , 236 , 53 , 19 , 15 , 252 , 246 , 81 , 237 , 169 , 42 , 219 , 18 , 32 , 52] , [9 , 50 , 43 , 223 , 76 , 108 , 146 , 164 , 3 , 6 , 118 , 136 , 69 , 180 , 17 , 33 , 50 , 66 , 253 , 163 , 222 , 239 , 29 , 4 , 0 , 9 , 143 , 162 , 157 , 196 , 42 , 10 , 37 , 160 , 245 , 101 , 212 , 223 , 100 , 9 , 183 , 244 , 141 , 151 , 87 , 241 , 240 , 244 , 109 , 90 , 24 , 95 , 204 , 109 , 255 , 40 , 22 , 151 , 100 , 215 , 17 , 221 , 203 , 113] , [34 , 101 , 54 , 1 , 33 , 144 , 72 , 18 , 180 , 136 , 29 , 111 , 253 , 3 , 75 , 166 , 117 , 111 , 33 , 39 , 161 , 203 , 238 , 87 , 171 , 140 , 220 , 46 , 158 , 66 , 232 , 5 , 21 , 199 , 117 , 43 , 29 , 67 , 10 , 244 , 185 , 13 , 71 , 253 , 204 , 10 , 120 , 152 , 254 , 50 , 178 , 39 , 105 , 136 , 110 , 237 , 67 , 210 , 116 , 15 , 70 , 154 , 135 , 156] , [17 , 215 , 5 , 205 , 156 , 89 , 128 , 154 , 152 , 12 , 106 , 140 , 217 , 196 , 33 , 97 , 212 , 218 , 47 , 80 , 220 , 77 , 35 , 116 , 27 , 32 , 183 , 115 , 63 , 189 , 230 , 79 , 40 , 237 , 49 , 85 , 37 , 20 , 186 , 128 , 94 , 222 , 99 , 2 , 4 , 105 , 77 , 202 , 220 , 83 , 80 , 245 , 26 , 107 , 75 , 242 , 47 , 139 , 157 , 175 , 18 , 205 , 169 , 8] , [42 , 117 , 183 , 73 , 98 , 148 , 197 , 201 , 93 , 12 , 53 , 246 , 46 , 186 , 70 , 201 , 56 , 209 , 160 , 45 , 221 , 68 , 174 , 178 , 215 , 39 , 112 , 157 , 152 , 240 , 63 , 79 , 6 , 204 , 150 , 68 , 13 , 234 , 80 , 127 , 138 , 99 , 215 , 171 , 200 , 175 , 207 , 39 , 7 , 251 , 216 , 82 , 146 , 161 , 201 , 114 , 21 , 251 , 170 , 107 , 219 , 125 , 35 , 134] , [44 , 201 , 91 , 12 , 124 , 52 , 89 , 66 , 238 , 169 , 182 , 108 , 234 , 249 , 77 , 145 , 69 , 255 , 125 , 162 , 194 , 122 , 77 , 160 , 183 , 24 , 179 , 199 , 84 , 254 , 176 , 5 , 4 , 193 , 184 , 38 , 123 , 166 , 250 , 227 , 66 , 43 , 93 , 186 , 207 , 209 , 193 , 55 , 241 , 180 , 124 , 195 , 21 , 85 , 13 , 59 , 99 , 107 , 200 , 178 , 187 , 164 , 119 , 81] , [30 , 32 , 165 , 174 , 31 , 92 , 79 , 57 , 27 , 163 , 232 , 92 , 134 , 193 , 67 , 122 , 95 , 69 , 215 , 49 , 196 , 69 , 156 , 94 , 95 , 28 , 186 , 93 , 159 , 9 , 73 , 5 , 35 , 152 , 68 , 60 , 113 , 21 , 11 , 114 , 19 , 53 , 172 , 118 , 6 , 7 , 31 , 145 , 154 , 133 , 85 , 76 , 32 , 247 , 213 , 242 , 174 , 237 , 9 , 39 , 202 , 202 , 118 , 172] , [35 , 215 , 169 , 66 , 125 , 152 , 99 , 8 , 208 , 232 , 15 , 28 , 237 , 68 , 188 , 64 , 54 , 89 , 195 , 239 , 193 , 225 , 53 , 7 , 40 , 182 , 162 , 142 , 245 , 199 , 193 , 131 , 9 , 27 , 236 , 217 , 147 , 18 , 21 , 145 , 251 , 117 , 146 , 126 , 207 , 128 , 12 , 75 , 65 , 144 , 242 , 213 , 106 , 45 , 226 , 82 , 18 , 167 , 38 , 27 , 176 , 217 , 33 , 161]] , }"
    }
  ],
  "instructions": [
    {
      "name": "shieldedTransferFirst",
      "docs": [
        "Saves the provided message in a temporary PDA."
      ],
      "accounts": [
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
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "shieldedTransferClose",
      "docs": [
        "Close the temporary PDA. Should be used when we don't intend to perform",
        "the second transfer and want to reclaim the funds."
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "shieldedTransferSecond",
      "docs": [
        "Stores the provided message in a compressed account, closes the",
        "temporary PDA."
      ],
      "accounts": [
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
          "name": "transactionMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "rpcRecipientSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "senderSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Verifier config pda which needs to exist."
          ]
        },
        {
          "name": "logWrapper",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "eventMerkleTree",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "verifierState",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "msg",
            "type": "bytes"
          }
        ]
      }
    },
    {
      "name": "instructionDataShieldedTransferFirst",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "message",
            "type": "bytes"
          }
        ]
      }
    },
    {
      "name": "instructionDataShieldedTransferSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proofA",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofB",
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          },
          {
            "name": "proofC",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "inputNullifier",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "publicAmountSol",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "rootIndex",
            "type": "u64"
          },
          {
            "name": "rpcFee",
            "type": "u64"
          },
          {
            "name": "encryptedUtxos",
            "type": {
              "array": [
                "u8",
                256
              ]
            }
          }
        ]
      }
    },
    {
      "name": "zKtransactionMasp2MainProofInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "root",
            "type": "u8"
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "txIntegrityHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPubkey",
            "type": "u8"
          },
          {
            "name": "inputNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "inPrivateKey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inBlinding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inAppDataHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inPathIndices",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inPathElements",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    18
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "inIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "outAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "outPubkey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outBlinding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outAppDataHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "outPoolType",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outVerifierPubkey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inPoolType",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inVerifierPubkey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "transactionVersion",
            "type": "u8"
          },
          {
            "name": "assetPubkeys",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "internalTxIntegrityHash",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "zKtransactionMasp2MainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "root",
            "type": "u8"
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "txIntegrityHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPubkey",
            "type": "u8"
          },
          {
            "name": "inputNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          }
        ]
      }
    },
    {
      "name": "instructionDataLightInstructionTransactionMasp2MainSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "root",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicAmountSpl",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "txIntegrityHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicAmountSol",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicMintPubkey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "inputNullifier",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                2
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                2
              ]
            }
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "NoopProgram",
      "msg": "The provided program is not the noop program."
    },
    {
      "code": 6001,
      "name": "MessageTooLarge",
      "msg": "Message too large, the limit per one method call is 1024 bytes."
    },
    {
      "code": 6002,
      "name": "VerifierStateNoSpace",
      "msg": "Cannot allocate more space for the verifier state account (message too large)."
    }
  ]
};
