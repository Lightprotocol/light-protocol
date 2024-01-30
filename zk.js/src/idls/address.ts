export type Address = {
  "version": "0.3.1",
  "name": "address",
  "instructions": [
    {
      "name": "initializeAddressQueue",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "queue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "initializeAddressMerkleTree",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "insertAddresses",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "queue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "addresses",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
          }
        }
      ]
    },
    {
      "name": "updateAddressMerkleTree",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "queue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "changelogIndex",
          "type": "u16"
        },
        {
          "name": "queueIndex",
          "type": "u16"
        },
        {
          "name": "addressNextIndex",
          "type": "u16"
        },
        {
          "name": "addressNextValue",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "lowAddress",
          "type": {
            "defined": "RawIndexingElement<32>"
          }
        },
        {
          "name": "lowAddressNextValue",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "lowAddressProof",
          "type": {
            "array": [
              {
                "array": [
                  "u8",
                  32
                ]
              },
              22
            ]
          }
        },
        {
          "name": "nextAddressProof",
          "type": {
            "array": [
              "u8",
              128
            ]
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "addressQueueAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "queue",
            "type": {
              "array": [
                "u8",
                112008
              ]
            }
          }
        ]
      }
    },
    {
      "name": "addressMerkleTreeAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "merkleTree",
            "type": {
              "array": [
                "u8",
                2173568
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
      "name": "AddressQueueInsert",
      "msg": "Failed to insert an element into indexing queue"
    },
    {
      "code": 6001,
      "name": "AddressQueueDequeue",
      "msg": "Failed to dequeue an element from indexing queue"
    },
    {
      "code": 6002,
      "name": "AddressMerkleTreeInitialize",
      "msg": "Failed to initialize address Merkle tree"
    },
    {
      "code": 6003,
      "name": "AddressMerkleTreeUpdate",
      "msg": "Failed to update the address Merkle tree"
    },
    {
      "code": 6004,
      "name": "InvalidIndex",
      "msg": "No element found under the given index in the queue"
    },
    {
      "code": 6005,
      "name": "BytesToBigint",
      "msg": "Failed to convert bytes to big integer"
    },
    {
      "code": 6006,
      "name": "IntegerOverflow",
      "msg": "Integer overflow"
    }
  ]
};

export const IDL: Address = {
  "version": "0.3.1",
  "name": "address",
  "instructions": [
    {
      "name": "initializeAddressQueue",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "queue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "initializeAddressMerkleTree",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "insertAddresses",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "queue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "addresses",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
          }
        }
      ]
    },
    {
      "name": "updateAddressMerkleTree",
      "accounts": [
        {
          "name": "authority",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "queue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "changelogIndex",
          "type": "u16"
        },
        {
          "name": "queueIndex",
          "type": "u16"
        },
        {
          "name": "addressNextIndex",
          "type": "u16"
        },
        {
          "name": "addressNextValue",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "lowAddress",
          "type": {
            "defined": "RawIndexingElement<32>"
          }
        },
        {
          "name": "lowAddressNextValue",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "lowAddressProof",
          "type": {
            "array": [
              {
                "array": [
                  "u8",
                  32
                ]
              },
              22
            ]
          }
        },
        {
          "name": "nextAddressProof",
          "type": {
            "array": [
              "u8",
              128
            ]
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "addressQueueAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "queue",
            "type": {
              "array": [
                "u8",
                112008
              ]
            }
          }
        ]
      }
    },
    {
      "name": "addressMerkleTreeAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "merkleTree",
            "type": {
              "array": [
                "u8",
                2173568
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
      "name": "AddressQueueInsert",
      "msg": "Failed to insert an element into indexing queue"
    },
    {
      "code": 6001,
      "name": "AddressQueueDequeue",
      "msg": "Failed to dequeue an element from indexing queue"
    },
    {
      "code": 6002,
      "name": "AddressMerkleTreeInitialize",
      "msg": "Failed to initialize address Merkle tree"
    },
    {
      "code": 6003,
      "name": "AddressMerkleTreeUpdate",
      "msg": "Failed to update the address Merkle tree"
    },
    {
      "code": 6004,
      "name": "InvalidIndex",
      "msg": "No element found under the given index in the queue"
    },
    {
      "code": 6005,
      "name": "BytesToBigint",
      "msg": "Failed to convert bytes to big integer"
    },
    {
      "code": 6006,
      "name": "IntegerOverflow",
      "msg": "Integer overflow"
    }
  ]
};
