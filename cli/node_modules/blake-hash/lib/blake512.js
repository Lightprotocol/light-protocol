const Blake = require('./blake')

const zo = Buffer.from([0x01])
const oo = Buffer.from([0x81])

function rot (v, i, j, n) {
  let hi = v[i * 2] ^ v[j * 2]
  let lo = v[i * 2 + 1] ^ v[j * 2 + 1]

  if (n >= 32) {
    lo = lo ^ hi
    hi = lo ^ hi
    lo = lo ^ hi
    n -= 32
  }

  if (n === 0) {
    v[i * 2] = hi >>> 0
    v[i * 2 + 1] = lo >>> 0
  } else {
    v[i * 2] = ((hi >>> n) | (lo << (32 - n))) >>> 0
    v[i * 2 + 1] = ((lo >>> n) | (hi << (32 - n))) >>> 0
  }
}

function g (v, m, i, a, b, c, d, e) {
  const sigma = Blake.sigma
  const u512 = Blake.u512
  let lo

  // v[a] += (m[sigma[i][e]] ^ u512[sigma[i][e+1]]) + v[b];
  lo = v[a * 2 + 1] + ((m[sigma[i][e] * 2 + 1] ^ u512[sigma[i][e + 1] * 2 + 1]) >>> 0) + v[b * 2 + 1]
  v[a * 2] = (v[a * 2] + ((m[sigma[i][e] * 2] ^ u512[sigma[i][e + 1] * 2]) >>> 0) + v[b * 2] + ~~(lo / 0x0100000000)) >>> 0
  v[a * 2 + 1] = lo >>> 0

  // v[d] = ROT( v[d] ^ v[a],32);
  rot(v, d, a, 32)

  // v[c] += v[d];
  lo = v[c * 2 + 1] + v[d * 2 + 1]
  v[c * 2] = (v[c * 2] + v[d * 2] + ~~(lo / 0x0100000000)) >>> 0
  v[c * 2 + 1] = lo >>> 0

  // v[b] = ROT( v[b] ^ v[c],25);
  rot(v, b, c, 25)

  // v[a] += (m[sigma[i][e+1]] ^ u512[sigma[i][e]])+v[b];
  lo = v[a * 2 + 1] + ((m[sigma[i][e + 1] * 2 + 1] ^ u512[sigma[i][e] * 2 + 1]) >>> 0) + v[b * 2 + 1]
  v[a * 2] = (v[a * 2] + ((m[sigma[i][e + 1] * 2] ^ u512[sigma[i][e] * 2]) >>> 0) + v[b * 2] + ~~(lo / 0x0100000000)) >>> 0
  v[a * 2 + 1] = lo >>> 0

  // v[d] = ROT( v[d] ^ v[a],16);
  rot(v, d, a, 16)

  // v[c] += v[d];
  lo = v[c * 2 + 1] + v[d * 2 + 1]
  v[c * 2] = (v[c * 2] + v[d * 2] + ~~(lo / 0x0100000000)) >>> 0
  v[c * 2 + 1] = lo >>> 0

  // v[b] = ROT( v[b] ^ v[c],11)
  rot(v, b, c, 11)
}

module.exports = class Blake512 extends Blake {
  constructor () {
    super()

    this._h = [
      0x6a09e667, 0xf3bcc908, 0xbb67ae85, 0x84caa73b,
      0x3c6ef372, 0xfe94f82b, 0xa54ff53a, 0x5f1d36f1,
      0x510e527f, 0xade682d1, 0x9b05688c, 0x2b3e6c1f,
      0x1f83d9ab, 0xfb41bd6b, 0x5be0cd19, 0x137e2179
    ]

    this._s = [0, 0, 0, 0, 0, 0, 0, 0]

    this._block = Buffer.alloc(128)
    this._blockOffset = 0
    this._length = [0, 0, 0, 0]

    this._nullt = false

    this._zo = zo
    this._oo = oo
  }

  _compress () {
    const u512 = Blake.u512
    const v = new Array(32)
    const m = new Array(32)
    let i

    for (i = 0; i < 32; ++i) m[i] = this._block.readUInt32BE(i * 4)
    for (i = 0; i < 16; ++i) v[i] = this._h[i] >>> 0
    for (i = 16; i < 24; ++i) v[i] = (this._s[i - 16] ^ u512[i - 16]) >>> 0
    for (i = 24; i < 32; ++i) v[i] = u512[i - 16]

    if (!this._nullt) {
      v[24] = (v[24] ^ this._length[1]) >>> 0
      v[25] = (v[25] ^ this._length[0]) >>> 0
      v[26] = (v[26] ^ this._length[1]) >>> 0
      v[27] = (v[27] ^ this._length[0]) >>> 0
      v[28] = (v[28] ^ this._length[3]) >>> 0
      v[29] = (v[29] ^ this._length[2]) >>> 0
      v[30] = (v[30] ^ this._length[3]) >>> 0
      v[31] = (v[31] ^ this._length[2]) >>> 0
    }

    for (i = 0; i < 16; ++i) {
      /* column step */
      g(v, m, i, 0, 4, 8, 12, 0)
      g(v, m, i, 1, 5, 9, 13, 2)
      g(v, m, i, 2, 6, 10, 14, 4)
      g(v, m, i, 3, 7, 11, 15, 6)
      /* diagonal step */
      g(v, m, i, 0, 5, 10, 15, 8)
      g(v, m, i, 1, 6, 11, 12, 10)
      g(v, m, i, 2, 7, 8, 13, 12)
      g(v, m, i, 3, 4, 9, 14, 14)
    }

    for (i = 0; i < 16; ++i) {
      this._h[(i % 8) * 2] = (this._h[(i % 8) * 2] ^ v[i * 2]) >>> 0
      this._h[(i % 8) * 2 + 1] = (this._h[(i % 8) * 2 + 1] ^ v[i * 2 + 1]) >>> 0
    }

    for (i = 0; i < 8; ++i) {
      this._h[i * 2] = (this._h[i * 2] ^ this._s[(i % 4) * 2]) >>> 0
      this._h[i * 2 + 1] = (this._h[i * 2 + 1] ^ this._s[(i % 4) * 2 + 1]) >>> 0
    }
  }

  _padding () {
    const len = this._length.slice()
    len[0] += this._blockOffset * 8
    this._lengthCarry(len)

    const msglen = Buffer.alloc(16)
    for (let i = 0; i < 4; ++i) msglen.writeUInt32BE(len[3 - i], i * 4)

    if (this._blockOffset === 111) {
      this._length[0] -= 8
      this.update(this._oo)
    } else {
      if (this._blockOffset < 111) {
        if (this._blockOffset === 0) this._nullt = true
        this._length[0] -= (111 - this._blockOffset) * 8
        this.update(Blake.padding.slice(0, 111 - this._blockOffset))
      } else {
        this._length[0] -= (128 - this._blockOffset) * 8
        this.update(Blake.padding.slice(0, 128 - this._blockOffset))
        this._length[0] -= 111 * 8
        this.update(Blake.padding.slice(1, 1 + 111))
        this._nullt = true
      }

      this.update(this._zo)
      this._length[0] -= 8
    }

    this._length[0] -= 128
    this.update(msglen)
  }

  digest () {
    this._padding()

    const buffer = Buffer.alloc(64)
    for (let i = 0; i < 16; ++i) buffer.writeUInt32BE(this._h[i], i * 4)
    return buffer
  }
}
