package prover

import "fmt"

func SetupCircuit(circuit string, treeDepth uint32, numberOfUtxos uint32) (*ProvingSystem, error) {
	if circuit == "inclusion" {
		return SetupInclusion(treeDepth, numberOfUtxos)
	} else if circuit == "non-inclusion" {
		return SetupNonInclusion(treeDepth, numberOfUtxos)
	} else if circuit == "combined" {
		return nil, fmt.Errorf("TODO: implement combine circuit: %s", circuit)
	} else {
		return nil, fmt.Errorf("invalid circuit: %s", circuit)
	}
}
