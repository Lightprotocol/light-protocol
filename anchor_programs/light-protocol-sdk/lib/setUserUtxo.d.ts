export declare function setUserUtxos(connection: any, recipientEncryptionKeypair: any, shieldedKeypair: any, ekpN: any, skpN: any): Promise<{
    unspentUtxos: any[];
    userBalance: number;
    nextIndex: number;
}>;
