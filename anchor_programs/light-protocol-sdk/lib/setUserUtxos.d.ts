export declare function setUserUtxos(connection: any, recipientEncryptionKeypair: any, shieldedKeypair: any): Promise<{
    unspentUtxos: any[];
    userBalance: number;
    nextIndex: number;
}>;
