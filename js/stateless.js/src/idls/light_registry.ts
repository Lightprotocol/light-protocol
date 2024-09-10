export type LightRegistry = {
    version: '1.0.0';
    name: 'light_registry';
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
                    isMut: false;
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
                        type: 'u32';
                    },
                    {
                        name: 'networkFee';
                        type: {
                            option: 'u64';
                        };
                    },
                    {
                        name: 'rolloverThreshold';
                        type: 'u64';
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
    ];
};

export const IDL: LightRegistry = {
    version: '1.0.0',
    name: 'light_registry',
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
                    isMut: false,
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
                        type: 'u32',
                    },
                    {
                        name: 'networkFee',
                        type: {
                            option: 'u64',
                        },
                    },
                    {
                        name: 'rolloverThreshold',
                        type: 'u64',
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
    ],
};