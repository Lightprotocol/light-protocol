package poseidon

import (
	"github.com/consensys/gnark/frontend"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type cfg struct {
	RF        int
	RP        int
	constants [][]frontend.Variable
	mds       [][]frontend.Variable
}

var CFG_2 = cfg{
	RF:        8,
	RP:        56,
	constants: CONSTANTS_2,
	mds:       MDS_2,
}
var CFG_3 = cfg{
	RF:        8,
	RP:        57,
	constants: CONSTANTS_3,
	mds:       MDS_3,
}
var CFG_4 = cfg{
	RF:        8,
	RP:        56,
	constants: CONSTANTS_4,
	mds:       MDS_4,
}

func cfgFor(t int) *cfg {
	switch t {
	case 2:
		return &CFG_2
	case 3:
		return &CFG_3
	case 4:
		return &CFG_4
	}
	panic("Poseidon: unsupported arg count")
}

type Poseidon1 struct {
	In frontend.Variable
}

func (g Poseidon1) DefineGadget(api frontend.API) interface{} {
	inp := []frontend.Variable{0, g.In}
	return abstractor.Call1(api, poseidon{inp})[0]
}

type Poseidon2 struct {
	In1, In2 frontend.Variable
}

func (g Poseidon2) DefineGadget(api frontend.API) interface{} {
	inp := []frontend.Variable{0, g.In1, g.In2}
	return abstractor.Call1(api, poseidon{inp})[0]
}

type Poseidon3 struct {
	In1, In2, In3 frontend.Variable
}

func (g Poseidon3) DefineGadget(api frontend.API) interface{} {
	inp := []frontend.Variable{0, g.In1, g.In2, g.In3}
	result := abstractor.Call1(api, poseidon{inp})[0]
	return result
}

type poseidon struct {
	Inputs []frontend.Variable
}

func (g poseidon) DefineGadget(api frontend.API) interface{} {
	state := g.Inputs
	cfg := cfgFor(len(state))
	for i := 0; i < cfg.RF/2; i += 1 {
		state = abstractor.Call1(api, fullRound{state, cfg.constants[i]})
		api.Println("state after fullRound", i, state)
	}
	for i := 0; i < cfg.RP; i += 1 {
		state = abstractor.Call1(api, halfRound{state, cfg.constants[cfg.RF/2+i]})
		api.Println("state after halfRound", i, state)
	}
	for i := 0; i < cfg.RF/2; i += 1 {
		state = abstractor.Call1(api, fullRound{state, cfg.constants[cfg.RF/2+cfg.RP+i]})
		api.Println("state after fullRound", i, state)
	}
	return state
}

type sbox struct {
	Inp frontend.Variable
}

func (s sbox) DefineGadget(api frontend.API) interface{} {
	v2 := api.Mul(s.Inp, s.Inp)
	v4 := api.Mul(v2, v2)
	r := api.Mul(s.Inp, v4)
	return r
}

type mds struct {
	Inp []frontend.Variable
}

func (m mds) DefineGadget(api frontend.API) interface{} {
	var mds = make([]frontend.Variable, len(m.Inp))
	cfg := cfgFor(len(m.Inp))
	for i := 0; i < len(m.Inp); i += 1 {
		var sum frontend.Variable = 0
		for j := 0; j < len(m.Inp); j += 1 {
			sum = api.Add(sum, api.Mul(m.Inp[j], cfg.mds[i][j]))
		}
		mds[i] = sum
	}
	return mds
}

type halfRound struct {
	Inp    []frontend.Variable
	Consts []frontend.Variable
}

func (h halfRound) DefineGadget(api frontend.API) interface{} {
	for i := 0; i < len(h.Inp); i += 1 {
		h.Inp[i] = api.Add(h.Inp[i], h.Consts[i])
	}
	h.Inp[0] = abstractor.Call(api, sbox{h.Inp[0]})
	return abstractor.Call1(api, mds{h.Inp})
}

type fullRound struct {
	Inp    []frontend.Variable
	Consts []frontend.Variable
}

func (h fullRound) DefineGadget(api frontend.API) interface{} {
	for i := 0; i < len(h.Inp); i += 1 {
		h.Inp[i] = api.Add(h.Inp[i], h.Consts[i])
	}
	for i := 0; i < len(h.Inp); i += 1 {
		h.Inp[i] = abstractor.Call(api, sbox{h.Inp[i]})
	}
	return abstractor.Call1(api, mds{h.Inp})
}
