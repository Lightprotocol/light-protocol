/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/sdk_anchor_test.json`.
 */
export type SdkAnchorTest = {
  "address": "2tzfijPBGbrR5PboyFUFKzfEoLTwdDSHUjANCw929wyt",
  "metadata": {
    "name": "sdkAnchorTest",
    "version": "0.7.0",
    "spec": "0.1.0",
    "description": "Test program for Light SDK and Light Macros"
  },
  "instructions": [
    {
      "name": "closeCompressedAccount",
      "discriminator": [
        55,
        108,
        99,
        108,
        119,
        228,
        247,
        203
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": {
            "defined": {
              "name": "validityProof"
            }
          }
        },
        {
          "name": "myCompressedAccount",
          "type": {
            "defined": {
              "name": "myCompressedAccount"
            }
          }
        },
        {
          "name": "accountMeta",
          "type": {
            "defined": {
              "name": "compressedAccountMeta"
            }
          }
        }
      ]
    },
    {
      "name": "closeCompressedAccountPermanent",
      "discriminator": [
        117,
        145,
        242,
        98,
        46,
        187,
        118,
        125
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": {
            "defined": {
              "name": "validityProof"
            }
          }
        },
        {
          "name": "accountMeta",
          "type": {
            "defined": {
              "name": "compressedAccountMetaBurn"
            }
          }
        }
      ]
    },
    {
      "name": "closeCompressedAccountV2",
      "discriminator": [
        12,
        21,
        104,
        30,
        185,
        99,
        10,
        30
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": {
            "defined": {
              "name": "validityProof"
            }
          }
        },
        {
          "name": "myCompressedAccount",
          "type": {
            "defined": {
              "name": "myCompressedAccount"
            }
          }
        },
        {
          "name": "accountMeta",
          "type": {
            "defined": {
              "name": "compressedAccountMeta"
            }
          }
        }
      ]
    },
    {
      "name": "createCompressedAccount",
      "discriminator": [
        74,
        87,
        131,
        150,
        204,
        209,
        66,
        94
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": {
            "defined": {
              "name": "validityProof"
            }
          }
        },
        {
          "name": "addressTreeInfo",
          "type": {
            "defined": {
              "name": "packedAddressTreeInfo"
            }
          }
        },
        {
          "name": "outputTreeIndex",
          "type": "u8"
        },
        {
          "name": "name",
          "type": "string"
        }
      ]
    },
    {
      "name": "createCompressedAccountV2",
      "discriminator": [
        16,
        69,
        137,
        87,
        207,
        37,
        81,
        138
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": {
            "defined": {
              "name": "validityProof"
            }
          }
        },
        {
          "name": "addressTreeInfo",
          "type": {
            "defined": {
              "name": "packedAddressTreeInfo"
            }
          }
        },
        {
          "name": "outputTreeIndex",
          "type": "u8"
        },
        {
          "name": "name",
          "type": "string"
        }
      ]
    },
    {
      "name": "reinitClosedAccount",
      "discriminator": [
        100,
        26,
        249,
        27,
        243,
        0,
        206,
        64
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": {
            "defined": {
              "name": "validityProof"
            }
          }
        },
        {
          "name": "accountMeta",
          "type": {
            "defined": {
              "name": "compressedAccountMeta"
            }
          }
        }
      ]
    },
    {
      "name": "updateCompressedAccount",
      "discriminator": [
        3,
        98,
        6,
        60,
        116,
        45,
        88,
        166
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": {
            "defined": {
              "name": "validityProof"
            }
          }
        },
        {
          "name": "myCompressedAccount",
          "type": {
            "defined": {
              "name": "myCompressedAccount"
            }
          }
        },
        {
          "name": "accountMeta",
          "type": {
            "defined": {
              "name": "compressedAccountMeta"
            }
          }
        },
        {
          "name": "nestedData",
          "type": {
            "defined": {
              "name": "nestedData"
            }
          }
        }
      ]
    },
    {
      "name": "updateCompressedAccountV2",
      "discriminator": [
        100,
        134,
        47,
        184,
        220,
        7,
        96,
        236
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": {
            "defined": {
              "name": "validityProof"
            }
          }
        },
        {
          "name": "myCompressedAccount",
          "type": {
            "defined": {
              "name": "myCompressedAccount"
            }
          }
        },
        {
          "name": "accountMeta",
          "type": {
            "defined": {
              "name": "compressedAccountMeta"
            }
          }
        },
        {
          "name": "nestedData",
          "type": {
            "defined": {
              "name": "nestedData"
            }
          }
        }
      ]
    },
    {
      "name": "withoutCompressedAccount",
      "discriminator": [
        68,
        84,
        81,
        196,
        24,
        131,
        208,
        209
      ],
      "accounts": [
        {
          "name": "signer",
          "writable": true,
          "signer": true
        },
        {
          "name": "myRegularAccount",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  109,
                  112,
                  114,
                  101,
                  115,
                  115,
                  101,
                  100
                ]
              },
              {
                "kind": "arg",
                "path": "name"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "name",
          "type": "string"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "myRegularAccount",
      "discriminator": [
        186,
        181,
        76,
        117,
        61,
        130,
        63,
        14
      ]
    }
  ],
  "events": [
    {
      "name": "myCompressedAccount",
      "discriminator": [
        147,
        40,
        99,
        80,
        53,
        44,
        10,
        210
      ]
    }
  ],
  "types": [
    {
      "name": "compressedAccountMeta",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "treeInfo",
            "docs": [
              "Merkle tree context."
            ],
            "type": {
              "defined": {
                "name": "packedStateTreeInfo"
              }
            }
          },
          {
            "name": "address",
            "docs": [
              "Address."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "outputStateTreeIndex",
            "docs": [
              "Output merkle tree index."
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "compressedAccountMetaBurn",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "treeInfo",
            "docs": [
              "State Merkle tree context."
            ],
            "type": {
              "defined": {
                "name": "packedStateTreeInfo"
              }
            }
          },
          {
            "name": "address",
            "docs": [
              "Address."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "compressedProof",
      "repr": {
        "kind": "c"
      },
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "a",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "b",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "c",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "myCompressedAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "name",
            "type": "string"
          },
          {
            "name": "nested",
            "type": {
              "defined": {
                "name": "nestedData"
              }
            }
          }
        ]
      }
    },
    {
      "name": "myRegularAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "name",
            "type": "string"
          }
        ]
      }
    },
    {
      "name": "nestedData",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "one",
            "type": "u16"
          },
          {
            "name": "two",
            "type": "u16"
          },
          {
            "name": "three",
            "type": "u16"
          },
          {
            "name": "four",
            "type": "u16"
          },
          {
            "name": "five",
            "type": "u16"
          },
          {
            "name": "six",
            "type": "u16"
          },
          {
            "name": "seven",
            "type": "u16"
          },
          {
            "name": "eight",
            "type": "u16"
          },
          {
            "name": "nine",
            "type": "u16"
          },
          {
            "name": "ten",
            "type": "u16"
          },
          {
            "name": "eleven",
            "type": "u16"
          },
          {
            "name": "twelve",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "packedAddressTreeInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "addressMerkleTreePubkeyIndex",
            "type": "u8"
          },
          {
            "name": "addressQueuePubkeyIndex",
            "type": "u8"
          },
          {
            "name": "rootIndex",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "packedStateTreeInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "rootIndex",
            "type": "u16"
          },
          {
            "name": "proveByIndex",
            "type": "bool"
          },
          {
            "name": "merkleTreePubkeyIndex",
            "type": "u8"
          },
          {
            "name": "queuePubkeyIndex",
            "type": "u8"
          },
          {
            "name": "leafIndex",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "validityProof",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "option": {
              "defined": {
                "name": "compressedProof"
              }
            }
          }
        ]
      }
    }
  ]
};
