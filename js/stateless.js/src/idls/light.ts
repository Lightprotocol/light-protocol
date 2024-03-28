export type Light = {
    version: '0.3.0';
    name: 'light';
    constants: [
        {
            name: 'AUTHORITY_PDA_SEED';
            type: 'bytes';
            value: '[97, 117, 116, 104, 111, 114, 105, 116, 121]';
        },
        {
            name: 'CPI_AUTHORITY_PDA_SEED';
            type: 'bytes';
            value: '[99, 112, 105, 95, 97, 117, 116, 104, 111, 114, 105, 116, 121]';
        },
    ];
    instructions: [
        {
            name: 'initializeGovernanceAuthority';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authorityPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'authority';
                    type: 'publicKey';
                },
                {
                    name: 'rewards';
                    type: {
                        vec: 'u64';
                    };
                },
                {
                    name: 'bump';
                    type: 'u8';
                },
            ];
        },
        {
            name: 'updateGovernanceAuthorityReward';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authorityPda';
                    isMut: true;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'reward';
                    type: 'u64';
                },
                {
                    name: 'index';
                    type: 'u64';
                },
            ];
        },
        {
            name: 'updateGovernanceAuthority';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authorityPda';
                    isMut: true;
                    isSigner: false;
                },
            ];
            args: [
                {
                    name: 'bump';
                    type: 'u8';
                },
                {
                    name: 'newAuthority';
                    type: 'publicKey';
                },
            ];
        },
        {
            name: 'registerSystemProgram';
            accounts: [
                {
                    name: 'authority';
                    isMut: true;
                    isSigner: true;
                },
                {
                    name: 'authorityPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'cpiAuthority';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'groupPda';
                    isMut: true;
                    isSigner: false;
                },
                {
                    name: 'accountCompressionProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'systemProgram';
                    isMut: false;
                    isSigner: false;
                },
                {
                    name: 'registeredProgramPda';
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
                    name: 'programId';
                    type: 'publicKey';
                },
            ];
        },
    ];
    accounts: [
        {
            name: 'lightGovernanceAuthority';
            type: {
                kind: 'struct';
                fields: [
                    {
                        name: 'authority';
                        type: 'publicKey';
                    },
                    {
                        name: 'bump';
                        type: 'u8';
                    },
                    {
                        name: 'padding';
                        type: {
                            array: ['u8', 7];
                        };
                    },
                    {
                        name: 'rewards';
                        type: {
                            vec: 'u64';
                        };
                    },
                ];
            };
        },
    ];
    errors: [
        {
            code: 6000;
            name: 'SumCheckFailed';
            msg: 'Sum check failed';
        },
    ];
};

export const IDL: Light = {
    version: '0.3.0',
    name: 'light',
    constants: [
        {
            name: 'AUTHORITY_PDA_SEED',
            type: 'bytes',
            value: '[97, 117, 116, 104, 111, 114, 105, 116, 121]',
        },
        {
            name: 'CPI_AUTHORITY_PDA_SEED',
            type: 'bytes',
            value: '[99, 112, 105, 95, 97, 117, 116, 104, 111, 114, 105, 116, 121]',
        },
    ],
    instructions: [
        {
            name: 'initializeGovernanceAuthority',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authorityPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'authority',
                    type: 'publicKey',
                },
                {
                    name: 'rewards',
                    type: {
                        vec: 'u64',
                    },
                },
                {
                    name: 'bump',
                    type: 'u8',
                },
            ],
        },
        {
            name: 'updateGovernanceAuthorityReward',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authorityPda',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'reward',
                    type: 'u64',
                },
                {
                    name: 'index',
                    type: 'u64',
                },
            ],
        },
        {
            name: 'updateGovernanceAuthority',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authorityPda',
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: 'bump',
                    type: 'u8',
                },
                {
                    name: 'newAuthority',
                    type: 'publicKey',
                },
            ],
        },
        {
            name: 'registerSystemProgram',
            accounts: [
                {
                    name: 'authority',
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: 'authorityPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'cpiAuthority',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'groupPda',
                    isMut: true,
                    isSigner: false,
                },
                {
                    name: 'accountCompressionProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'systemProgram',
                    isMut: false,
                    isSigner: false,
                },
                {
                    name: 'registeredProgramPda',
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
                    name: 'programId',
                    type: 'publicKey',
                },
            ],
        },
    ],
    accounts: [
        {
            name: 'lightGovernanceAuthority',
            type: {
                kind: 'struct',
                fields: [
                    {
                        name: 'authority',
                        type: 'publicKey',
                    },
                    {
                        name: 'bump',
                        type: 'u8',
                    },
                    {
                        name: 'padding',
                        type: {
                            array: ['u8', 7],
                        },
                    },
                    {
                        name: 'rewards',
                        type: {
                            vec: 'u64',
                        },
                    },
                ],
            },
        },
    ],
    errors: [
        {
            code: 6000,
            name: 'SumCheckFailed',
            msg: 'Sum check failed',
        },
    ],
};
