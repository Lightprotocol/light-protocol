var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
export const parseInputsToBytesArray = function (data) {
  var mydata = JSON.parse(data.toString());
  for (var i in mydata) {
    mydata[i] = Array.from(
      leInt2Buff(unstringifyBigInts(mydata[i]), 32)
    ).reverse();
  }
  return mydata;
};
