pragma circom 2.1.4;
// Checks that that for every i there is only one index == 1 for all assets
template CheckIndices(n, nInAssets, nAssets) {
  signal input indices[n][nInAssets][nAssets];
  signal input amounts[n][nInAssets];

  for (var i = 0; i < n; i++) {
      for (var j = 0; j < nInAssets; j++) {
          var varSumIndices = 0;
          for (var z = 0; z < nAssets; z++) {
              varSumIndices += indices[i][j][z];
              // all indices are 0 or 1
              indices[i][j][z] * (1 - indices[i][j][z]) === 0;
          }
          // only one index for one asset is 1
          varSumIndices * (1 - varSumIndices)=== 0;
          // if amount != 0 there should be one an asset assigned to it
          varSumIndices * amounts[i][j] === amounts[i][j];
      }
  }
}