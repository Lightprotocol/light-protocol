package common

type CircuitType string

const (
	CombinedCircuitType           CircuitType = "combined"
	InclusionCircuitType          CircuitType = "inclusion"
	NonInclusionCircuitType       CircuitType = "non-inclusion"
	BatchAppendCircuitType        CircuitType = "append"
	BatchUpdateCircuitType        CircuitType = "update"
	BatchAddressAppendCircuitType CircuitType = "address-append"
	MaspUtxoCircuitType           CircuitType = "masp-utxo"
	MaspTreeCircuitType           CircuitType = "masp-tree"
	MaspBundleCircuitType         CircuitType = "masp-bundle"
)

func IsMaspCircuit(t CircuitType) bool {
	switch t {
	case MaspUtxoCircuitType, MaspTreeCircuitType, MaspBundleCircuitType:
		return true
	}
	return false
}

// JSON input structures (these are not in circuit_utils.go)
type InclusionProofInputsJSON struct {
	Root         string   `json:"root"`
	PathIndex    uint32   `json:"pathIndex"`
	PathElements []string `json:"pathElements"`
	Leaf         string   `json:"leaf"`
}

type NonInclusionProofInputsJSON struct {
	Root                                string   `json:"root"`
	Value                               string   `json:"value"`
	LeafLowerRangeValue                 string   `json:"leafLowerRangeValue"`
	LeafHigherRangeValue                string   `json:"leafHigherRangeValue"`
	LeafIndex                           uint32   `json:"leafIndex"`
	MerkleProofHashedIndexedElementLeaf []string `json:"merkleProofHashedIndexedElementLeaf"`
	IndexHashedIndexedElementLeaf       uint32   `json:"indexHashedIndexedElementLeaf"`
}
