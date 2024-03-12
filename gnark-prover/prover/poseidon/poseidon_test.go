package poseidon

import (
	"testing"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
	"github.com/reilabs/gnark-lean-extractor/v2/abstractor"
)

type TestPoseidonCircuit1 struct {
	Input frontend.Variable `gnark:"input"`
	Hash  frontend.Variable `gnark:",public"`
}

type TestPoseidonCircuit2 struct {
	Left  frontend.Variable `gnark:"left"`
	Right frontend.Variable `gnark:"right"`
	Hash  frontend.Variable `gnark:",public"`
}

type TestPoseidonCircuit3 struct {
	First  frontend.Variable `gnark:"first"`
	Second frontend.Variable `gnark:"second"`
	Third  frontend.Variable `gnark:"third"`
	Hash   frontend.Variable `gnark:",public"`
}

func (circuit *TestPoseidonCircuit1) Define(api frontend.API) error {
	poseidon := abstractor.Call(api, Poseidon1{circuit.Input})
	api.AssertIsEqual(circuit.Hash, poseidon)
	return nil
}

func (circuit *TestPoseidonCircuit2) Define(api frontend.API) error {
	poseidon := abstractor.Call(api, Poseidon2{circuit.Left, circuit.Right})
	api.AssertIsEqual(circuit.Hash, poseidon)
	return nil
}

func (circuit *TestPoseidonCircuit3) Define(api frontend.API) error {
	poseidon := abstractor.Call(api, Poseidon3{circuit.First, circuit.Second, circuit.Third})
	api.AssertIsEqual(circuit.Hash, poseidon)
	return nil
}

func TestPoseidon(t *testing.T) {
	assert := test.NewAssert(t)

	var circuit1 TestPoseidonCircuit1
	assert.ProverSucceeded(&circuit1, &TestPoseidonCircuit1{
		Input: 0,
		Hash:  hex("0x2a09a9fd93c590c26b91effbb2499f07e8f7aa12e2b4940a3aed2411cb65e11c"),
	}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254))

	assert.ProverSucceeded(&circuit1, &TestPoseidonCircuit1{
		Input: 1,
		Hash:  hex("0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133"),
	}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254))

	assert.ProverSucceeded(&circuit1, &TestPoseidonCircuit1{
		Input: 2,
		Hash:  hex("0x131d73cf6b30079aca0dff6a561cd0ee50b540879abe379a25a06b24bde2bebd"),
	}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254))

	var circuit2 TestPoseidonCircuit2

	assert.ProverSucceeded(&circuit2, &TestPoseidonCircuit2{
		Left:  0,
		Right: 0,
		Hash:  hex("0x2098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b64864"),
	}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254))

	assert.ProverSucceeded(&circuit2, &TestPoseidonCircuit2{
		Left:  0,
		Right: 1,
		Hash:  hex("0x1bd20834f5de9830c643778a2e88a3a1363c8b9ac083d36d75bf87c49953e65e"),
	}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254))

	assert.ProverSucceeded(&circuit2, &TestPoseidonCircuit2{
		Left:  1,
		Right: 1,
		Hash:  hex("0x7af346e2d304279e79e0a9f3023f771294a78acb70e73f90afe27cad401e81"),
	}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254))

	assert.ProverSucceeded(&circuit2, &TestPoseidonCircuit2{
		Left:  1,
		Right: 2,
		Hash:  hex("0x115cc0f5e7d690413df64c6b9662e9cf2a3617f2743245519e19607a4417189a"),
	}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254))

	assert.ProverSucceeded(&circuit2, &TestPoseidonCircuit2{
		Left:  31213,
		Right: 132,
		Hash:  hex("0x303f59cd0831b5633bcda50514521b33776b5d4280eb5868ba1dbbe2e4d76ab5"),
	}, test.WithBackends(backend.GROTH16), test.WithCurves(ecc.BN254))

}
