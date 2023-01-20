const Blake = require('./blake')

const zo = Buffer.from([0x01])
const oo = Buffer.from([0x81])

const rot = (x, n) => ((x << (32 - n)) | (x >>> n)) >>> 0

function g (v, m, i, a, b, c, d, e) {
  const sigma = Blake.sigma
  const u256 = Blake.u256

  v[a] = (v[a] + ((m[sigma[i][e]] ^ u256[sigma[i][e + 1]]) >>> 0) + v[b]) >>> 0
  v[d] = rot(v[d] ^ v[a], 16)
  v[c] = (v[c] + v[d]) >>> 0
  v[b] = rot(v[b] ^ v[c], 12)
  v[a] = (v[a] + ((m[sigma[i][e + 1]] ^ u256[sigma[i][e]]) >>> 0) + v[b]) >>> 0
  v[d] = rot(v[d] ^ v[a], 8)
  v[c] = (v[c] + v[d]) >>> 0
  v[b] = rot(v[b] ^ v[c], 7)
}

module.exports = class Blake256 extends Blake {
  constructor () {
    super()

    this._h = [
      0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
      0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
    ]

    this._s = [0, 0, 0, 0]

    this._block = Buffer.alloc(64)
    this._blockOffset = 0
    this._length = [0, 0]

    this._nullt = false

    this._zo = zo
    this._oo = oo
  }

  _compress () {
    const u256 = Blake.u256
    const v = new Array(16)
    const m = new Array(16)
    let i

    for (i = 0; i < 16; ++i) m[i] = this._block.readUInt32BE(i * 4)
    for (i = 0; i < 8; ++i) v[i] = this._h[i] >>> 0
    for (i = 8; i < 12; ++i) v[i] = (this._s[i - 8] ^ u256[i - 8]) >>> 0
    for (i = 12; i < 16; ++i) v[i] = u256[i - 8]

    if (!this._nullt) {
      v[12] = (v[12] ^ this._length[0]) >>> 0
      v[13] = (v[13] ^ this._length[0]) >>> 0
      v[14] = (v[14] ^ this._length[1]) >>> 0
      v[15] = (v[15] ^ this._length[1]) >>> 0
    }

    for (i = 0; i < 14; ++i) {
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

    for (i = 0; i < 16; ++i) this._h[i % 8] = (this._h[i % 8] ^ v[i]) >>> 0
    for (i = 0; i < 8; ++i) this._h[i] = (this._h[i] ^ this._s[i % 4]) >>> 0
  }

  _padding () {
    let lo = this._length[0] + this._blockOffset * 8
    let hi = this._length[1]
    if (lo >= 0x0100000000) {
      lo -= 0x0100000000
      hi += 1
    }

    const msglen = Buffer.alloc(8)
    msglen.writeUInt32BE(hi, 0)
    msglen.writeUInt32BE(lo, 4)

    if (this._blockOffset === 55) {
      this._length[0] -= 8
      this.update(this._oo)
    } else {
      if (this._blockOffset < 55) {
        if (this._blockOffset === 0) this._nullt = true
        this._length[0] -= (55 - this._blockOffset) * 8
        this.update(Blake.padding.slice(0, 55 - this._blockOffset))
      } else {
        this._length[0] -= (64 - this._blockOffset) * 8
        this.update(Blake.padding.slice(0, 64 - this._blockOffset))
        this._length[0] -= 55 * 8
        this.update(Blake.padding.slice(1, 1 + 55))
        this._nullt = true
      }

      this.update(this._zo)
      this._length[0] -= 8
    }

    this._length[0] -= 64
    this.update(msglen)
  }

  digest () {
    this._padding()

    const buffer = Buffer.alloc(32)
    for (let i = 0; i < 8; ++i) buffer.writeUInt32BE(this._h[i], i * 4)
    return buffer
  }
}
