package prover

import "fmt"

func SetupCircuit(circuit string, treeDepth uint32, numberOfUtxos uint32) (*ProvingSystem, error) {
	if circuit == "inclusion" {
		return SetupInclusion(treeDepth, numberOfUtxos)
	} else {
		return nil, fmt.Errorf("invalid circuit: %s", circuit)
	}
}
