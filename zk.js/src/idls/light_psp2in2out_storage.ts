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
      "name": "VERIFYINGKEY_PRIVATE_TRANSACTION2_IN2_OUT_MAIN",
      "type": {
        "defined": "Groth16Verifyingkey"
      },
      "value": "Groth16Verifyingkey { nr_pubinputs : 12 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [3 , 131 , 93 , 180 , 18 , 180 , 142 , 134 , 206 , 57 , 208 , 199 , 1 , 111 , 151 , 184 , 99 , 17 , 85 , 12 , 57 , 229 , 122 , 190 , 9 , 212 , 149 , 67 , 151 , 213 , 83 , 218 , 1 , 153 , 134 , 52 , 236 , 229 , 248 , 8 , 220 , 157 , 140 , 176 , 81 , 199 , 54 , 77 , 195 , 37 , 41 , 174 , 197 , 134 , 213 , 232 , 196 , 215 , 98 , 40 , 140 , 140 , 59 , 0 , 26 , 132 , 148 , 218 , 74 , 229 , 242 , 98 , 147 , 229 , 253 , 221 , 153 , 228 , 88 , 135 , 164 , 153 , 199 , 147 , 223 , 213 , 232 , 225 , 185 , 33 , 22 , 112 , 190 , 161 , 222 , 235 , 34 , 241 , 105 , 41 , 148 , 198 , 204 , 170 , 245 , 6 , 139 , 92 , 64 , 178 , 158 , 91 , 127 , 184 , 106 , 103 , 55 , 179 , 145 , 102 , 104 , 54 , 50 , 218 , 169 , 140 , 80 , 158] , vk_ic : & [[4 , 63 , 226 , 92 , 100 , 183 , 102 , 89 , 0 , 175 , 240 , 54 , 193 , 216 , 179 , 163 , 99 , 172 , 242 , 128 , 67 , 249 , 250 , 233 , 94 , 52 , 181 , 60 , 110 , 247 , 65 , 174 , 17 , 121 , 230 , 223 , 215 , 138 , 8 , 38 , 107 , 154 , 211 , 30 , 35 , 134 , 115 , 192 , 173 , 35 , 72 , 251 , 140 , 202 , 96 , 168 , 232 , 7 , 5 , 83 , 206 , 29 , 171 , 217] , [34 , 47 , 205 , 251 , 205 , 37 , 7 , 120 , 188 , 154 , 33 , 185 , 118 , 104 , 156 , 6 , 62 , 42 , 194 , 197 , 41 , 178 , 170 , 6 , 53 , 113 , 105 , 136 , 248 , 195 , 227 , 225 , 46 , 6 , 165 , 36 , 174 , 185 , 85 , 0 , 210 , 227 , 69 , 211 , 226 , 124 , 171 , 0 , 249 , 35 , 239 , 53 , 106 , 21 , 211 , 10 , 234 , 124 , 91 , 61 , 208 , 20 , 31 , 111] , [10 , 5 , 26 , 152 , 150 , 138 , 93 , 234 , 56 , 193 , 23 , 212 , 26 , 97 , 98 , 81 , 150 , 127 , 133 , 122 , 187 , 174 , 99 , 156 , 84 , 68 , 247 , 232 , 65 , 215 , 97 , 39 , 33 , 43 , 10 , 242 , 33 , 201 , 73 , 91 , 235 , 216 , 93 , 159 , 0 , 134 , 185 , 241 , 38 , 158 , 135 , 121 , 188 , 217 , 162 , 32 , 174 , 194 , 196 , 234 , 231 , 185 , 190 , 128] , [11 , 44 , 166 , 190 , 255 , 162 , 201 , 215 , 68 , 244 , 179 , 33 , 37 , 111 , 164 , 29 , 162 , 58 , 83 , 220 , 43 , 32 , 212 , 45 , 89 , 129 , 241 , 145 , 32 , 168 , 231 , 207 , 36 , 158 , 132 , 239 , 140 , 167 , 98 , 128 , 156 , 0 , 98 , 201 , 237 , 37 , 137 , 64 , 120 , 100 , 196 , 216 , 167 , 190 , 116 , 171 , 19 , 76 , 241 , 104 , 66 , 162 , 60 , 231] , [40 , 60 , 71 , 210 , 48 , 194 , 16 , 104 , 43 , 113 , 24 , 63 , 109 , 74 , 89 , 108 , 160 , 27 , 4 , 181 , 115 , 68 , 226 , 83 , 76 , 230 , 63 , 23 , 217 , 207 , 227 , 112 , 8 , 177 , 217 , 156 , 231 , 148 , 39 , 145 , 237 , 38 , 54 , 100 , 15 , 125 , 175 , 253 , 107 , 178 , 206 , 51 , 193 , 133 , 54 , 141 , 162 , 29 , 255 , 128 , 216 , 18 , 125 , 90] , [19 , 20 , 207 , 214 , 117 , 143 , 59 , 149 , 216 , 65 , 10 , 114 , 181 , 51 , 184 , 208 , 133 , 9 , 144 , 211 , 74 , 158 , 82 , 16 , 67 , 138 , 158 , 17 , 123 , 76 , 159 , 147 , 20 , 169 , 79 , 50 , 224 , 5 , 31 , 117 , 108 , 151 , 108 , 171 , 3 , 217 , 231 , 198 , 173 , 183 , 114 , 92 , 188 , 162 , 242 , 164 , 4 , 191 , 159 , 46 , 117 , 135 , 1 , 207] , [15 , 179 , 233 , 208 , 93 , 54 , 160 , 43 , 75 , 59 , 160 , 17 , 222 , 142 , 20 , 164 , 5 , 111 , 245 , 17 , 81 , 163 , 52 , 54 , 6 , 138 , 42 , 228 , 241 , 123 , 120 , 130 , 4 , 133 , 171 , 163 , 243 , 62 , 192 , 57 , 108 , 55 , 192 , 255 , 62 , 225 , 178 , 66 , 126 , 165 , 226 , 101 , 32 , 179 , 102 , 201 , 209 , 203 , 208 , 115 , 35 , 14 , 88 , 138] , [44 , 26 , 55 , 42 , 120 , 94 , 71 , 250 , 134 , 140 , 179 , 6 , 247 , 104 , 182 , 193 , 195 , 162 , 105 , 63 , 83 , 108 , 247 , 249 , 162 , 53 , 178 , 17 , 228 , 30 , 37 , 243 , 28 , 15 , 4 , 252 , 181 , 215 , 143 , 186 , 76 , 239 , 98 , 203 , 59 , 52 , 161 , 252 , 189 , 215 , 238 , 53 , 248 , 45 , 211 , 148 , 167 , 117 , 221 , 110 , 135 , 223 , 41 , 134] , [27 , 198 , 144 , 131 , 115 , 162 , 202 , 254 , 239 , 96 , 122 , 173 , 191 , 17 , 94 , 103 , 198 , 241 , 179 , 49 , 30 , 139 , 176 , 253 , 32 , 214 , 116 , 196 , 5 , 188 , 229 , 58 , 32 , 103 , 128 , 132 , 56 , 32 , 173 , 209 , 15 , 61 , 194 , 174 , 240 , 224 , 133 , 175 , 173 , 141 , 68 , 37 , 32 , 22 , 160 , 246 , 114 , 165 , 225 , 172 , 40 , 39 , 134 , 61] , [24 , 219 , 50 , 220 , 173 , 12 , 132 , 210 , 12 , 66 , 9 , 145 , 183 , 37 , 122 , 212 , 254 , 137 , 59 , 131 , 230 , 38 , 53 , 212 , 57 , 220 , 75 , 132 , 226 , 64 , 125 , 218 , 44 , 139 , 211 , 220 , 74 , 174 , 133 , 126 , 230 , 80 , 140 , 45 , 189 , 69 , 162 , 115 , 241 , 154 , 78 , 234 , 142 , 42 , 199 , 171 , 87 , 48 , 72 , 182 , 191 , 25 , 57 , 155] , [45 , 155 , 31 , 224 , 16 , 20 , 85 , 205 , 24 , 6 , 22 , 237 , 82 , 51 , 16 , 150 , 135 , 96 , 57 , 44 , 227 , 54 , 21 , 166 , 133 , 235 , 57 , 102 , 7 , 80 , 223 , 43 , 13 , 169 , 48 , 142 , 24 , 137 , 15 , 250 , 67 , 201 , 145 , 111 , 182 , 197 , 179 , 63 , 188 , 250 , 3 , 246 , 184 , 144 , 75 , 143 , 127 , 191 , 128 , 69 , 240 , 147 , 9 , 167] , [23 , 220 , 75 , 182 , 78 , 59 , 87 , 150 , 127 , 63 , 45 , 232 , 210 , 128 , 117 , 101 , 234 , 56 , 82 , 98 , 95 , 94 , 157 , 10 , 151 , 246 , 204 , 2 , 99 , 96 , 107 , 51 , 22 , 192 , 112 , 241 , 251 , 151 , 57 , 185 , 93 , 26 , 21 , 160 , 31 , 99 , 127 , 193 , 128 , 180 , 141 , 218 , 107 , 247 , 208 , 47 , 164 , 239 , 150 , 17 , 72 , 254 , 147 , 196] , [44 , 33 , 68 , 109 , 60 , 143 , 80 , 16 , 253 , 125 , 141 , 159 , 109 , 29 , 181 , 71 , 80 , 158 , 1 , 161 , 221 , 98 , 177 , 105 , 161 , 228 , 253 , 102 , 78 , 156 , 195 , 29 , 2 , 43 , 71 , 76 , 140 , 186 , 151 , 46 , 126 , 181 , 99 , 190 , 119 , 141 , 98 , 38 , 36 , 167 , 157 , 77 , 1 , 62 , 249 , 212 , 88 , 179 , 208 , 90 , 92 , 116 , 48 , 189]] , }"
    }
  ],
  "instructions": [
    {
      "name": "compressedTransferFirst",
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
      "name": "compressedTransferClose",
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
      "name": "compressedTransferSecond",
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
      "name": "instructionDataCompressedTransferFirst",
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
      "name": "instructionDataCompressedTransferSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proofA",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "proofB",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofC",
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
            "name": "publicOutUtxoHash",
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
      "name": "zKprivateTransaction2In2OutMainProofInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
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
            "name": "publicOutUtxoHash",
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
            "name": "address",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "metaHash",
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
            "name": "inDataHash",
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
            "name": "nullifierLeafIndex",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "nullifierMerkleProof",
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
          }
        ]
      }
    },
    {
      "name": "zKprivateTransaction2In2OutMainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
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
            "name": "publicOutUtxoHash",
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
      "name": "instructionDataLightInstructionPrivateTransaction2In2OutMainSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
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
            "name": "publicNullifierRoot",
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
            "name": "publicOutUtxoHash",
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
      "name": "VERIFYINGKEY_PRIVATE_TRANSACTION2_IN2_OUT_MAIN",
      "type": {
        "defined": "Groth16Verifyingkey"
      },
      "value": "Groth16Verifyingkey { nr_pubinputs : 12 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [3 , 131 , 93 , 180 , 18 , 180 , 142 , 134 , 206 , 57 , 208 , 199 , 1 , 111 , 151 , 184 , 99 , 17 , 85 , 12 , 57 , 229 , 122 , 190 , 9 , 212 , 149 , 67 , 151 , 213 , 83 , 218 , 1 , 153 , 134 , 52 , 236 , 229 , 248 , 8 , 220 , 157 , 140 , 176 , 81 , 199 , 54 , 77 , 195 , 37 , 41 , 174 , 197 , 134 , 213 , 232 , 196 , 215 , 98 , 40 , 140 , 140 , 59 , 0 , 26 , 132 , 148 , 218 , 74 , 229 , 242 , 98 , 147 , 229 , 253 , 221 , 153 , 228 , 88 , 135 , 164 , 153 , 199 , 147 , 223 , 213 , 232 , 225 , 185 , 33 , 22 , 112 , 190 , 161 , 222 , 235 , 34 , 241 , 105 , 41 , 148 , 198 , 204 , 170 , 245 , 6 , 139 , 92 , 64 , 178 , 158 , 91 , 127 , 184 , 106 , 103 , 55 , 179 , 145 , 102 , 104 , 54 , 50 , 218 , 169 , 140 , 80 , 158] , vk_ic : & [[4 , 63 , 226 , 92 , 100 , 183 , 102 , 89 , 0 , 175 , 240 , 54 , 193 , 216 , 179 , 163 , 99 , 172 , 242 , 128 , 67 , 249 , 250 , 233 , 94 , 52 , 181 , 60 , 110 , 247 , 65 , 174 , 17 , 121 , 230 , 223 , 215 , 138 , 8 , 38 , 107 , 154 , 211 , 30 , 35 , 134 , 115 , 192 , 173 , 35 , 72 , 251 , 140 , 202 , 96 , 168 , 232 , 7 , 5 , 83 , 206 , 29 , 171 , 217] , [34 , 47 , 205 , 251 , 205 , 37 , 7 , 120 , 188 , 154 , 33 , 185 , 118 , 104 , 156 , 6 , 62 , 42 , 194 , 197 , 41 , 178 , 170 , 6 , 53 , 113 , 105 , 136 , 248 , 195 , 227 , 225 , 46 , 6 , 165 , 36 , 174 , 185 , 85 , 0 , 210 , 227 , 69 , 211 , 226 , 124 , 171 , 0 , 249 , 35 , 239 , 53 , 106 , 21 , 211 , 10 , 234 , 124 , 91 , 61 , 208 , 20 , 31 , 111] , [10 , 5 , 26 , 152 , 150 , 138 , 93 , 234 , 56 , 193 , 23 , 212 , 26 , 97 , 98 , 81 , 150 , 127 , 133 , 122 , 187 , 174 , 99 , 156 , 84 , 68 , 247 , 232 , 65 , 215 , 97 , 39 , 33 , 43 , 10 , 242 , 33 , 201 , 73 , 91 , 235 , 216 , 93 , 159 , 0 , 134 , 185 , 241 , 38 , 158 , 135 , 121 , 188 , 217 , 162 , 32 , 174 , 194 , 196 , 234 , 231 , 185 , 190 , 128] , [11 , 44 , 166 , 190 , 255 , 162 , 201 , 215 , 68 , 244 , 179 , 33 , 37 , 111 , 164 , 29 , 162 , 58 , 83 , 220 , 43 , 32 , 212 , 45 , 89 , 129 , 241 , 145 , 32 , 168 , 231 , 207 , 36 , 158 , 132 , 239 , 140 , 167 , 98 , 128 , 156 , 0 , 98 , 201 , 237 , 37 , 137 , 64 , 120 , 100 , 196 , 216 , 167 , 190 , 116 , 171 , 19 , 76 , 241 , 104 , 66 , 162 , 60 , 231] , [40 , 60 , 71 , 210 , 48 , 194 , 16 , 104 , 43 , 113 , 24 , 63 , 109 , 74 , 89 , 108 , 160 , 27 , 4 , 181 , 115 , 68 , 226 , 83 , 76 , 230 , 63 , 23 , 217 , 207 , 227 , 112 , 8 , 177 , 217 , 156 , 231 , 148 , 39 , 145 , 237 , 38 , 54 , 100 , 15 , 125 , 175 , 253 , 107 , 178 , 206 , 51 , 193 , 133 , 54 , 141 , 162 , 29 , 255 , 128 , 216 , 18 , 125 , 90] , [19 , 20 , 207 , 214 , 117 , 143 , 59 , 149 , 216 , 65 , 10 , 114 , 181 , 51 , 184 , 208 , 133 , 9 , 144 , 211 , 74 , 158 , 82 , 16 , 67 , 138 , 158 , 17 , 123 , 76 , 159 , 147 , 20 , 169 , 79 , 50 , 224 , 5 , 31 , 117 , 108 , 151 , 108 , 171 , 3 , 217 , 231 , 198 , 173 , 183 , 114 , 92 , 188 , 162 , 242 , 164 , 4 , 191 , 159 , 46 , 117 , 135 , 1 , 207] , [15 , 179 , 233 , 208 , 93 , 54 , 160 , 43 , 75 , 59 , 160 , 17 , 222 , 142 , 20 , 164 , 5 , 111 , 245 , 17 , 81 , 163 , 52 , 54 , 6 , 138 , 42 , 228 , 241 , 123 , 120 , 130 , 4 , 133 , 171 , 163 , 243 , 62 , 192 , 57 , 108 , 55 , 192 , 255 , 62 , 225 , 178 , 66 , 126 , 165 , 226 , 101 , 32 , 179 , 102 , 201 , 209 , 203 , 208 , 115 , 35 , 14 , 88 , 138] , [44 , 26 , 55 , 42 , 120 , 94 , 71 , 250 , 134 , 140 , 179 , 6 , 247 , 104 , 182 , 193 , 195 , 162 , 105 , 63 , 83 , 108 , 247 , 249 , 162 , 53 , 178 , 17 , 228 , 30 , 37 , 243 , 28 , 15 , 4 , 252 , 181 , 215 , 143 , 186 , 76 , 239 , 98 , 203 , 59 , 52 , 161 , 252 , 189 , 215 , 238 , 53 , 248 , 45 , 211 , 148 , 167 , 117 , 221 , 110 , 135 , 223 , 41 , 134] , [27 , 198 , 144 , 131 , 115 , 162 , 202 , 254 , 239 , 96 , 122 , 173 , 191 , 17 , 94 , 103 , 198 , 241 , 179 , 49 , 30 , 139 , 176 , 253 , 32 , 214 , 116 , 196 , 5 , 188 , 229 , 58 , 32 , 103 , 128 , 132 , 56 , 32 , 173 , 209 , 15 , 61 , 194 , 174 , 240 , 224 , 133 , 175 , 173 , 141 , 68 , 37 , 32 , 22 , 160 , 246 , 114 , 165 , 225 , 172 , 40 , 39 , 134 , 61] , [24 , 219 , 50 , 220 , 173 , 12 , 132 , 210 , 12 , 66 , 9 , 145 , 183 , 37 , 122 , 212 , 254 , 137 , 59 , 131 , 230 , 38 , 53 , 212 , 57 , 220 , 75 , 132 , 226 , 64 , 125 , 218 , 44 , 139 , 211 , 220 , 74 , 174 , 133 , 126 , 230 , 80 , 140 , 45 , 189 , 69 , 162 , 115 , 241 , 154 , 78 , 234 , 142 , 42 , 199 , 171 , 87 , 48 , 72 , 182 , 191 , 25 , 57 , 155] , [45 , 155 , 31 , 224 , 16 , 20 , 85 , 205 , 24 , 6 , 22 , 237 , 82 , 51 , 16 , 150 , 135 , 96 , 57 , 44 , 227 , 54 , 21 , 166 , 133 , 235 , 57 , 102 , 7 , 80 , 223 , 43 , 13 , 169 , 48 , 142 , 24 , 137 , 15 , 250 , 67 , 201 , 145 , 111 , 182 , 197 , 179 , 63 , 188 , 250 , 3 , 246 , 184 , 144 , 75 , 143 , 127 , 191 , 128 , 69 , 240 , 147 , 9 , 167] , [23 , 220 , 75 , 182 , 78 , 59 , 87 , 150 , 127 , 63 , 45 , 232 , 210 , 128 , 117 , 101 , 234 , 56 , 82 , 98 , 95 , 94 , 157 , 10 , 151 , 246 , 204 , 2 , 99 , 96 , 107 , 51 , 22 , 192 , 112 , 241 , 251 , 151 , 57 , 185 , 93 , 26 , 21 , 160 , 31 , 99 , 127 , 193 , 128 , 180 , 141 , 218 , 107 , 247 , 208 , 47 , 164 , 239 , 150 , 17 , 72 , 254 , 147 , 196] , [44 , 33 , 68 , 109 , 60 , 143 , 80 , 16 , 253 , 125 , 141 , 159 , 109 , 29 , 181 , 71 , 80 , 158 , 1 , 161 , 221 , 98 , 177 , 105 , 161 , 228 , 253 , 102 , 78 , 156 , 195 , 29 , 2 , 43 , 71 , 76 , 140 , 186 , 151 , 46 , 126 , 181 , 99 , 190 , 119 , 141 , 98 , 38 , 36 , 167 , 157 , 77 , 1 , 62 , 249 , 212 , 88 , 179 , 208 , 90 , 92 , 116 , 48 , 189]] , }"
    }
  ],
  "instructions": [
    {
      "name": "compressedTransferFirst",
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
      "name": "compressedTransferClose",
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
      "name": "compressedTransferSecond",
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
      "name": "instructionDataCompressedTransferFirst",
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
      "name": "instructionDataCompressedTransferSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proofA",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "proofB",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofC",
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
            "name": "publicOutUtxoHash",
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
      "name": "zKprivateTransaction2In2OutMainProofInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
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
            "name": "publicOutUtxoHash",
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
            "name": "address",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "metaHash",
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
            "name": "inDataHash",
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
            "name": "nullifierLeafIndex",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "nullifierMerkleProof",
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
          }
        ]
      }
    },
    {
      "name": "zKprivateTransaction2In2OutMainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
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
            "name": "publicOutUtxoHash",
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
      "name": "instructionDataLightInstructionPrivateTransaction2In2OutMainSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
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
            "name": "publicNullifierRoot",
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
            "name": "publicOutUtxoHash",
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
