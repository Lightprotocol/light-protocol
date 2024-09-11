export type LightRegistry = {
    version: '1.0.0';
    name: 'light_registry';
    accounts: [
        {
            "name": "ProtocolConfigPda",
            "type": {
              "kind": "struct",
              "fields": [
                {
                  "name": "authority",
                  "type": "publicKey"
                },
                {
                  "name": "bump",
                  "type": "u8"
                },
                {
                  "name": "config",
                  "type": {
                    "defined": "ProtocolConfig"
                  }
                }
              ]
            }
          },
    ],
    instructions: [
        {
            name: 'initializeStateMerkleTree';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'merkleTree';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'queue';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'registeredProgramPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'cpiAuthority';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'protocolConfigPda';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'cpiContextAccount';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'lightSystemProgram';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'programOwner';
                    type: {
                        option: 'publicKey';
                    };
                },
                {
                    name: 'forester';
                    type: {
                        option: 'publicKey';
                    };
                },
                {
                    name: 'merkleTreeConfig';
                    type: {
                        defined: 'StateMerkleTreeConfig';
                    };
                },
                {
                    name: 'queueConfig';
                    type: {
                        defined: 'NullifierQueueConfig';
                    };
                },
            ];
        },
    ];
    types: [
        {
            name: 'StateMerkleTreeConfig';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'height';
                        type: 'u32';
                    },
                    {
                        name: 'changelogSize';
                        type: 'u64';
                    },
                    {
                        name: 'rootsSize';
                        type: 'u64';
                    },
                    {
                        name: 'canopyDepth';
                        type: 'u64';
                    },
                    {
                        name: 'networkFee';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'rolloverThreshold';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'closeThreshold';
                        type: {
                            option: 'u64';
                        };
                    },
                ];
            };
        },
        {
            name: 'NullifierQueueConfig';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'capacity';
                        type: 'u32';
                    },
                    {
                        name: 'sequenceThreshold';
                        type: 'u64';
                    },
                    {
                        name: 'networkFee';
                        type: {
                            option: 'u64';
                        };
                    },
                ];
            };
        },
        {
            name: 'ProtocolConfig';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'genesisSlot';
                        type: 'u64';
                    },
                    {
                        name: 'minWeight';
                        type: 'u64';
                    },
                    {
                        name: 'slotLength';
                        type: 'u64';
                    },
                    {
                        name: 'registrationPhaseLength';
                        type: 'u64';
                    },
                    {
                        name: 'activePhaseLength';
                        type: 'u64';
                    },
                    {
                        name: 'reportWorkPhaseLength';
                        type: 'u64';
                    },
                    {
                        name: 'networkFee';
                        type: 'u64';
                    },
                    {
                        name: 'cpiContextSize';
                        type: 'u64';
                    },
                    {
                        name: 'finalizeCounterLimit';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolder';
                        type: 'publicKey';
                    },
                    {
                        name: 'placeHolderA';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolderB';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolderC';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolderD';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolderE';
                        type: 'u64';
                    },
                    {
                        name: 'placeHolderF';
                        type: 'u64';
                    },
                ];
            };
        },
    ];
};

export const IDL: LightRegistry = {
    version: '1.0.0',
    name: 'light_registry',
      accounts: [
        {
            "name": "ProtocolConfigPda",
            "type": {
              "kind": "struct",
              "fields": [
                {
                  "name": "authority",
                  "type": "publicKey"
                },
                {
                  "name": "bump",
                  "type": "u8"
                },
                {
                  "name": "config",
                  "type": {
                    "defined": "ProtocolConfig"
                  }
                }
              ]
            }
          },
    ],
    instructions: [
        {
            name: 'initializeStateMerkleTree',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'merkleTree',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'queue',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'registeredProgramPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'cpiAuthority',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'protocolConfigPda',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'cpiContextAccount',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'lightSystemProgram',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'bump',
                    type: 'u8',
                },
                {
                    name: 'programOwner',
                    type: {
                        option: 'publicKey',
                    },
                },
                {
                    name: 'forester',
                    type: {
                        option: 'publicKey',
                    },
                },
                {
                    name: 'merkleTreeConfig',
                    type: {
                        defined: 'StateMerkleTreeConfig',
                    },
                },
                {
                    name: 'queueConfig',
                    type: {
                        defined: 'NullifierQueueConfig',
                    },
                },
            ],
        },
    ],
    types: [
        {
            name: 'StateMerkleTreeConfig',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'height',
                        type: 'u32',
                    },
                    {
                        name: 'changelogSize',
                        type: 'u64',
                    },
                    {
                        name: 'rootsSize',
                        type: 'u64',
                    },
                    {
                        name: 'canopyDepth',
                        type: 'u64',
                    },
                    {
                        name: 'networkFee',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'rolloverThreshold',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'closeThreshold',
                        type: {
                            option: 'u64',
                        },
                    },
                ],
            },
        },
        {
            name: 'NullifierQueueConfig',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'capacity',
                        type: 'u32',
                    },
                    {
                        name: 'sequenceThreshold',
                        type: 'u64',
                    },
                    {
                        name: 'networkFee',
                        type: {
                            option: 'u64',
                        },
                    },
                ],
            },
        },
        {
            name: 'ProtocolConfig',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'genesisSlot',
                        type: 'u64',
                    },
                    {
                        name: 'minWeight',
                        type: 'u64',
                    },
                    {
                        name: 'slotLength',
                        type: 'u64',
                    },
                    {
                        name: 'registrationPhaseLength',
                        type: 'u64',
                    },
                    {
                        name: 'activePhaseLength',
                        type: 'u64',
                    },
                    {
                        name: 'reportWorkPhaseLength',
                        type: 'u64',
                    },
                    {
                        name: 'networkFee',
                        type: 'u64',
                    },
                    {
                        name: 'cpiContextSize',
                        type: 'u64',
                    },
                    {
                        name: 'finalizeCounterLimit',
                        type: 'u64',
                    },
                    {
                        name: 'placeHolder',
                        type: 'publicKey',
                    },
                    {
                        name: 'placeHolderA',
                        type: 'u64',
                    },
                    {
                        name: 'placeHolderB',
                        type: 'u64',
                    },
                    {
                        name: 'placeHolderC',
                        type: 'u64',
                    },
                    {
                        name: 'placeHolderD',
                        type: 'u64',
                    },
                    {
                        name: 'placeHolderE',
                        type: 'u64',
                    },
                    {
                        name: 'placeHolderF',
                        type: 'u64',
                    },
                ],
            },
        },
    ],
};