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
      "value": "Groth16Verifyingkey { nr_pubinputs : 9 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [48 , 90 , 197 , 128 , 82 , 194 , 46 , 50 , 223 , 150 , 110 , 150 , 157 , 27 , 9 , 1 , 115 , 104 , 208 , 50 , 220 , 254 , 63 , 189 , 70 , 137 , 251 , 213 , 111 , 241 , 187 , 85 , 45 , 108 , 14 , 141 , 208 , 34 , 214 , 182 , 74 , 239 , 22 , 12 , 210 , 69 , 212 , 58 , 50 , 232 , 164 , 106 , 239 , 197 , 226 , 116 , 240 , 29 , 108 , 224 , 215 , 166 , 223 , 2 , 2 , 116 , 48 , 66 , 251 , 248 , 199 , 136 , 178 , 3 , 154 , 151 , 255 , 27 , 95 , 31 , 221 , 192 , 83 , 35 , 29 , 134 , 65 , 115 , 184 , 107 , 83 , 220 , 124 , 181 , 66 , 163 , 14 , 63 , 236 , 40 , 69 , 167 , 99 , 132 , 3 , 186 , 199 , 143 , 71 , 37 , 76 , 148 , 253 , 24 , 69 , 207 , 39 , 150 , 84 , 68 , 52 , 185 , 11 , 199 , 110 , 173 , 79 , 182] , vk_ic : & [[6 , 57 , 4 , 80 , 167 , 69 , 203 , 156 , 103 , 157 , 241 , 255 , 168 , 156 , 72 , 140 , 61 , 139 , 128 , 254 , 127 , 123 , 83 , 100 , 234 , 114 , 13 , 229 , 255 , 209 , 238 , 14 , 8 , 107 , 34 , 156 , 207 , 86 , 78 , 204 , 185 , 30 , 84 , 101 , 103 , 232 , 91 , 88 , 23 , 124 , 127 , 69 , 214 , 167 , 68 , 218 , 221 , 65 , 91 , 82 , 245 , 13 , 149 , 88] , [32 , 242 , 195 , 87 , 34 , 194 , 233 , 57 , 225 , 123 , 197 , 179 , 26 , 199 , 167 , 227 , 44 , 95 , 251 , 59 , 210 , 212 , 21 , 1 , 196 , 79 , 79 , 210 , 91 , 243 , 185 , 177 , 27 , 127 , 160 , 176 , 53 , 214 , 247 , 172 , 147 , 85 , 224 , 96 , 62 , 242 , 133 , 1 , 183 , 232 , 195 , 89 , 101 , 35 , 245 , 85 , 14 , 102 , 54 , 144 , 245 , 206 , 61 , 254] , [21 , 114 , 3 , 28 , 231 , 226 , 186 , 160 , 154 , 63 , 23 , 192 , 57 , 95 , 233 , 59 , 83 , 39 , 32 , 155 , 143 , 244 , 89 , 93 , 127 , 153 , 18 , 192 , 105 , 21 , 72 , 165 , 30 , 34 , 63 , 79 , 202 , 69 , 67 , 134 , 13 , 174 , 85 , 16 , 12 , 188 , 189 , 100 , 138 , 1 , 162 , 204 , 34 , 53 , 197 , 91 , 238 , 213 , 3 , 9 , 158 , 21 , 56 , 111] , [46 , 89 , 202 , 166 , 152 , 66 , 152 , 171 , 34 , 185 , 2 , 38 , 12 , 232 , 24 , 174 , 55 , 148 , 11 , 202 , 175 , 72 , 129 , 202 , 132 , 188 , 189 , 69 , 37 , 160 , 122 , 113 , 2 , 53 , 25 , 234 , 93 , 108 , 156 , 220 , 116 , 12 , 176 , 164 , 156 , 201 , 166 , 208 , 52 , 85 , 213 , 236 , 149 , 233 , 182 , 62 , 5 , 174 , 234 , 174 , 46 , 122 , 182 , 227] , [3 , 162 , 68 , 228 , 35 , 55 , 83 , 14 , 58 , 133 , 233 , 212 , 142 , 226 , 149 , 137 , 235 , 45 , 192 , 129 , 73 , 30 , 228 , 168 , 164 , 192 , 25 , 137 , 154 , 42 , 119 , 155 , 10 , 202 , 185 , 233 , 96 , 140 , 126 , 192 , 165 , 231 , 164 , 14 , 152 , 50 , 10 , 94 , 229 , 93 , 183 , 136 , 78 , 13 , 86 , 211 , 74 , 28 , 247 , 132 , 210 , 40 , 102 , 170] , [31 , 65 , 52 , 44 , 83 , 185 , 12 , 177 , 76 , 183 , 249 , 255 , 136 , 6 , 225 , 59 , 176 , 182 , 148 , 239 , 54 , 32 , 23 , 130 , 161 , 4 , 248 , 166 , 233 , 182 , 163 , 161 , 14 , 179 , 25 , 151 , 108 , 212 , 10 , 224 , 47 , 211 , 213 , 13 , 24 , 93 , 5 , 174 , 59 , 19 , 159 , 128 , 180 , 216 , 115 , 165 , 64 , 73 , 174 , 40 , 170 , 87 , 2 , 37] , [12 , 121 , 7 , 175 , 117 , 24 , 129 , 221 , 35 , 65 , 90 , 46 , 121 , 40 , 105 , 221 , 220 , 128 , 223 , 23 , 144 , 185 , 223 , 189 , 136 , 122 , 170 , 7 , 57 , 162 , 231 , 82 , 34 , 65 , 42 , 3 , 193 , 174 , 244 , 122 , 223 , 190 , 217 , 123 , 69 , 95 , 134 , 187 , 129 , 111 , 228 , 124 , 222 , 123 , 206 , 64 , 78 , 161 , 187 , 54 , 228 , 175 , 218 , 105] , [22 , 80 , 89 , 53 , 120 , 211 , 222 , 32 , 196 , 237 , 127 , 197 , 217 , 40 , 173 , 144 , 110 , 69 , 160 , 84 , 14 , 79 , 208 , 42 , 97 , 149 , 99 , 66 , 2 , 242 , 206 , 160 , 3 , 96 , 81 , 253 , 40 , 94 , 174 , 108 , 195 , 250 , 112 , 194 , 53 , 122 , 188 , 53 , 13 , 130 , 19 , 178 , 148 , 217 , 207 , 67 , 36 , 52 , 115 , 167 , 137 , 92 , 142 , 151] , [41 , 179 , 239 , 224 , 4 , 146 , 76 , 138 , 210 , 168 , 217 , 154 , 26 , 187 , 174 , 126 , 229 , 117 , 193 , 115 , 57 , 156 , 142 , 239 , 34 , 207 , 66 , 146 , 159 , 170 , 123 , 95 , 32 , 214 , 97 , 53 , 9 , 153 , 12 , 60 , 41 , 217 , 69 , 18 , 78 , 232 , 250 , 184 , 251 , 243 , 211 , 48 , 222 , 185 , 115 , 75 , 148 , 235 , 152 , 192 , 66 , 149 , 120 , 198] , [11 , 246 , 203 , 156 , 252 , 171 , 207 , 231 , 121 , 177 , 151 , 98 , 73 , 117 , 181 , 4 , 44 , 167 , 52 , 28 , 151 , 3 , 6 , 154 , 66 , 197 , 112 , 87 , 29 , 229 , 51 , 105 , 9 , 254 , 153 , 240 , 123 , 228 , 101 , 188 , 184 , 175 , 47 , 52 , 188 , 135 , 125 , 80 , 179 , 243 , 159 , 6 , 34 , 179 , 203 , 179 , 102 , 234 , 143 , 235 , 168 , 57 , 0 , 165]] , }"
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
            "name": "publicNullifier",
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
            "name": "publicUtxoHash",
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
            "name": "publicRoot",
            "type": "u8"
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicUtxoHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "assetPublicKeys",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "privatePublicDataHash",
            "type": "u8"
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
            "name": "inDataHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inVerifierPublicKey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "leafIndex",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "merkleProof",
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
            "name": "outOwner",
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
            "name": "outDataHash",
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
            "name": "outVerifierPublicKey",
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
      "name": "zKtransactionMasp2MainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicRoot",
            "type": "u8"
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicUtxoHash",
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
            "name": "publicRoot",
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
            "name": "publicDataHash",
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
            "name": "publicMintPublicKey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicNullifier",
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
            "name": "publicUtxoHash",
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
      "value": "Groth16Verifyingkey { nr_pubinputs : 9 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [48 , 90 , 197 , 128 , 82 , 194 , 46 , 50 , 223 , 150 , 110 , 150 , 157 , 27 , 9 , 1 , 115 , 104 , 208 , 50 , 220 , 254 , 63 , 189 , 70 , 137 , 251 , 213 , 111 , 241 , 187 , 85 , 45 , 108 , 14 , 141 , 208 , 34 , 214 , 182 , 74 , 239 , 22 , 12 , 210 , 69 , 212 , 58 , 50 , 232 , 164 , 106 , 239 , 197 , 226 , 116 , 240 , 29 , 108 , 224 , 215 , 166 , 223 , 2 , 2 , 116 , 48 , 66 , 251 , 248 , 199 , 136 , 178 , 3 , 154 , 151 , 255 , 27 , 95 , 31 , 221 , 192 , 83 , 35 , 29 , 134 , 65 , 115 , 184 , 107 , 83 , 220 , 124 , 181 , 66 , 163 , 14 , 63 , 236 , 40 , 69 , 167 , 99 , 132 , 3 , 186 , 199 , 143 , 71 , 37 , 76 , 148 , 253 , 24 , 69 , 207 , 39 , 150 , 84 , 68 , 52 , 185 , 11 , 199 , 110 , 173 , 79 , 182] , vk_ic : & [[6 , 57 , 4 , 80 , 167 , 69 , 203 , 156 , 103 , 157 , 241 , 255 , 168 , 156 , 72 , 140 , 61 , 139 , 128 , 254 , 127 , 123 , 83 , 100 , 234 , 114 , 13 , 229 , 255 , 209 , 238 , 14 , 8 , 107 , 34 , 156 , 207 , 86 , 78 , 204 , 185 , 30 , 84 , 101 , 103 , 232 , 91 , 88 , 23 , 124 , 127 , 69 , 214 , 167 , 68 , 218 , 221 , 65 , 91 , 82 , 245 , 13 , 149 , 88] , [32 , 242 , 195 , 87 , 34 , 194 , 233 , 57 , 225 , 123 , 197 , 179 , 26 , 199 , 167 , 227 , 44 , 95 , 251 , 59 , 210 , 212 , 21 , 1 , 196 , 79 , 79 , 210 , 91 , 243 , 185 , 177 , 27 , 127 , 160 , 176 , 53 , 214 , 247 , 172 , 147 , 85 , 224 , 96 , 62 , 242 , 133 , 1 , 183 , 232 , 195 , 89 , 101 , 35 , 245 , 85 , 14 , 102 , 54 , 144 , 245 , 206 , 61 , 254] , [21 , 114 , 3 , 28 , 231 , 226 , 186 , 160 , 154 , 63 , 23 , 192 , 57 , 95 , 233 , 59 , 83 , 39 , 32 , 155 , 143 , 244 , 89 , 93 , 127 , 153 , 18 , 192 , 105 , 21 , 72 , 165 , 30 , 34 , 63 , 79 , 202 , 69 , 67 , 134 , 13 , 174 , 85 , 16 , 12 , 188 , 189 , 100 , 138 , 1 , 162 , 204 , 34 , 53 , 197 , 91 , 238 , 213 , 3 , 9 , 158 , 21 , 56 , 111] , [46 , 89 , 202 , 166 , 152 , 66 , 152 , 171 , 34 , 185 , 2 , 38 , 12 , 232 , 24 , 174 , 55 , 148 , 11 , 202 , 175 , 72 , 129 , 202 , 132 , 188 , 189 , 69 , 37 , 160 , 122 , 113 , 2 , 53 , 25 , 234 , 93 , 108 , 156 , 220 , 116 , 12 , 176 , 164 , 156 , 201 , 166 , 208 , 52 , 85 , 213 , 236 , 149 , 233 , 182 , 62 , 5 , 174 , 234 , 174 , 46 , 122 , 182 , 227] , [3 , 162 , 68 , 228 , 35 , 55 , 83 , 14 , 58 , 133 , 233 , 212 , 142 , 226 , 149 , 137 , 235 , 45 , 192 , 129 , 73 , 30 , 228 , 168 , 164 , 192 , 25 , 137 , 154 , 42 , 119 , 155 , 10 , 202 , 185 , 233 , 96 , 140 , 126 , 192 , 165 , 231 , 164 , 14 , 152 , 50 , 10 , 94 , 229 , 93 , 183 , 136 , 78 , 13 , 86 , 211 , 74 , 28 , 247 , 132 , 210 , 40 , 102 , 170] , [31 , 65 , 52 , 44 , 83 , 185 , 12 , 177 , 76 , 183 , 249 , 255 , 136 , 6 , 225 , 59 , 176 , 182 , 148 , 239 , 54 , 32 , 23 , 130 , 161 , 4 , 248 , 166 , 233 , 182 , 163 , 161 , 14 , 179 , 25 , 151 , 108 , 212 , 10 , 224 , 47 , 211 , 213 , 13 , 24 , 93 , 5 , 174 , 59 , 19 , 159 , 128 , 180 , 216 , 115 , 165 , 64 , 73 , 174 , 40 , 170 , 87 , 2 , 37] , [12 , 121 , 7 , 175 , 117 , 24 , 129 , 221 , 35 , 65 , 90 , 46 , 121 , 40 , 105 , 221 , 220 , 128 , 223 , 23 , 144 , 185 , 223 , 189 , 136 , 122 , 170 , 7 , 57 , 162 , 231 , 82 , 34 , 65 , 42 , 3 , 193 , 174 , 244 , 122 , 223 , 190 , 217 , 123 , 69 , 95 , 134 , 187 , 129 , 111 , 228 , 124 , 222 , 123 , 206 , 64 , 78 , 161 , 187 , 54 , 228 , 175 , 218 , 105] , [22 , 80 , 89 , 53 , 120 , 211 , 222 , 32 , 196 , 237 , 127 , 197 , 217 , 40 , 173 , 144 , 110 , 69 , 160 , 84 , 14 , 79 , 208 , 42 , 97 , 149 , 99 , 66 , 2 , 242 , 206 , 160 , 3 , 96 , 81 , 253 , 40 , 94 , 174 , 108 , 195 , 250 , 112 , 194 , 53 , 122 , 188 , 53 , 13 , 130 , 19 , 178 , 148 , 217 , 207 , 67 , 36 , 52 , 115 , 167 , 137 , 92 , 142 , 151] , [41 , 179 , 239 , 224 , 4 , 146 , 76 , 138 , 210 , 168 , 217 , 154 , 26 , 187 , 174 , 126 , 229 , 117 , 193 , 115 , 57 , 156 , 142 , 239 , 34 , 207 , 66 , 146 , 159 , 170 , 123 , 95 , 32 , 214 , 97 , 53 , 9 , 153 , 12 , 60 , 41 , 217 , 69 , 18 , 78 , 232 , 250 , 184 , 251 , 243 , 211 , 48 , 222 , 185 , 115 , 75 , 148 , 235 , 152 , 192 , 66 , 149 , 120 , 198] , [11 , 246 , 203 , 156 , 252 , 171 , 207 , 231 , 121 , 177 , 151 , 98 , 73 , 117 , 181 , 4 , 44 , 167 , 52 , 28 , 151 , 3 , 6 , 154 , 66 , 197 , 112 , 87 , 29 , 229 , 51 , 105 , 9 , 254 , 153 , 240 , 123 , 228 , 101 , 188 , 184 , 175 , 47 , 52 , 188 , 135 , 125 , 80 , 179 , 243 , 159 , 6 , 34 , 179 , 203 , 179 , 102 , 234 , 143 , 235 , 168 , 57 , 0 , 165]] , }"
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
            "name": "publicNullifier",
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
            "name": "publicUtxoHash",
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
            "name": "publicRoot",
            "type": "u8"
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicUtxoHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "assetPublicKeys",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "privatePublicDataHash",
            "type": "u8"
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
            "name": "inDataHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inVerifierPublicKey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "leafIndex",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "merkleProof",
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
            "name": "outOwner",
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
            "name": "outDataHash",
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
            "name": "outVerifierPublicKey",
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
      "name": "zKtransactionMasp2MainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicRoot",
            "type": "u8"
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicUtxoHash",
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
            "name": "publicRoot",
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
            "name": "publicDataHash",
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
            "name": "publicMintPublicKey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicNullifier",
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
            "name": "publicUtxoHash",
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
