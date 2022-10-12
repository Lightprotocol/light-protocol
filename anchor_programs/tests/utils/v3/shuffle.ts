export const shuffle = function (utxos) {
    let currentIndex = utxos.length;
    let randomIndex;
    // While there remain elements to shuffle...
    while (0 !== currentIndex) {
        // Pick a remaining element...
        randomIndex = Math.floor(Math.random() * currentIndex);
        currentIndex--;
        [utxos[currentIndex], utxos[randomIndex]] = [
            utxos[randomIndex],
            utxos[currentIndex],
        ];
    }
    return utxos;
};
