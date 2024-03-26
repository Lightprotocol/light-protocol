package prover

import "fmt"

func SetupCircuit(circuit string, inclusionTreeDepth uint32, inclusionNumberOfUtxos uint32, nonInclusionTreeDepth uint32, nonInclusionNumberOfUtxos uint32) (*ProvingSystem, error) {
	if circuit == "inclusion" {
		return SetupInclusion(inclusionTreeDepth, inclusionNumberOfUtxos)
	} else if circuit == "non-inclusion" {
		return SetupNonInclusion(nonInclusionTreeDepth, nonInclusionNumberOfUtxos)
	} else if circuit == "combined" {
		return SetupCombined(inclusionTreeDepth, inclusionNumberOfUtxos, nonInclusionTreeDepth, nonInclusionNumberOfUtxos)
	} else {
		return nil, fmt.Errorf("invalid circuit: %s", circuit)
	}
}
