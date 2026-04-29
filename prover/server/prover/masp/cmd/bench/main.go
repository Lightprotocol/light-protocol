// bench compiles and times the MASP utxo_circuit and tree_circuit across a
// standard set of (N, M) shapes, printing constraint counts and phase
// timings. Shapes match bench-shielded-pool/p256/main.go so numbers are
// comparable across benches.
package main

import (
	"bytes"
	"flag"
	"fmt"
	"os"
	"time"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"

	"light/light-prover/prover/masp"
)

type shape struct{ N, M int }

var defaultShapes = []shape{
	{2, 2},
	{4, 4},
	{16, 2},
	{32, 2},
	{32, 32},
}

type phaseTimes struct {
	Constraints int
	Compile     time.Duration
	Setup       time.Duration
	Prove       time.Duration
	Verify      time.Duration
	ProofBytes  int
	PKBytes     int
	VKBytes     int
}

type rowResult struct {
	N, M int
	Utxo phaseTimes
	Tree phaseTimes
}

func main() {
	// --export DIR [N M]: emit gnark artifacts (.r1cs, .pk, .vk,
	// .witness.json) for the utxo + tree circuits at the given shape.
	// Defaults to 4/4 (reasonable device footprint).
	if len(os.Args) > 2 && os.Args[1] == "--export" {
		dir := os.Args[2]
		n, m := 4, 4
		if len(os.Args) >= 5 {
			fmt.Sscanf(os.Args[3], "%d", &n)
			fmt.Sscanf(os.Args[4], "%d", &m)
		}
		if err := os.MkdirAll(dir, 0o755); err != nil {
			panic(err)
		}
		w, err := masp.NewSampleWitness(n, m, masp.SampleMixed)
		if err != nil {
			fmt.Fprintln(os.Stderr, "build witness:", err)
			os.Exit(1)
		}
		base := fmt.Sprintf("%din_%dout", n, m)
		if err := exportCircuit(dir, "masp_"+base+"_utxo",
			masp.NewUtxoCircuit(n, m), w.UtxoAssignment()); err != nil {
			fmt.Fprintln(os.Stderr, "export utxo:", err)
			os.Exit(1)
		}
		if err := exportCircuit(dir, "masp_"+base+"_tree",
			masp.NewTreeCircuit(n), w.TreeAssignment()); err != nil {
			fmt.Fprintln(os.Stderr, "export tree:", err)
			os.Exit(1)
		}
		return
	}

	singleN := flag.Int("n", 0, "run only one shape with this many inputs (0 = run default suite)")
	singleM := flag.Int("m", 0, "outputs for single-shape run")
	flag.Parse()

	var shapes []shape
	if *singleN > 0 && *singleM > 0 {
		shapes = []shape{{*singleN, *singleM}}
	} else {
		shapes = defaultShapes
	}

	fmt.Printf("=== MASP Groth16 Benchmark (P-256 / BN254) ===\n\n")
	results := make([]rowResult, 0, len(shapes))
	for _, s := range shapes {
		fmt.Printf(">> N=%d in, M=%d out\n", s.N, s.M)
		w, err := masp.NewSampleWitness(s.N, s.M, masp.SampleMixed)
		if err != nil {
			fmt.Fprintf(os.Stderr, "build witness (%d/%d): %v\n", s.N, s.M, err)
			os.Exit(1)
		}
		ut := runCircuit("utxo_circuit", masp.NewUtxoCircuit(s.N, s.M), w.UtxoAssignment())
		tr := runCircuit("tree_circuit", masp.NewTreeCircuit(s.N), w.TreeAssignment())
		results = append(results, rowResult{N: s.N, M: s.M, Utxo: ut, Tree: tr})
		fmt.Println()
	}

	// Summary tables.
	printSummary(results)
}

func runCircuit(name string, circuit, assignment frontend.Circuit) phaseTimes {
	var pt phaseTimes

	fmt.Printf("  %s:\n", name)

	start := time.Now()
	ccs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, circuit)
	if err != nil {
		fmt.Printf("    Compile error: %v\n", err)
		return pt
	}
	pt.Compile = time.Since(start)
	pt.Constraints = ccs.GetNbConstraints()
	fmt.Printf("    Constraints: %d\n", pt.Constraints)
	fmt.Printf("    Compile:     %v\n", pt.Compile)

	start = time.Now()
	pk, vk, err := groth16.Setup(ccs)
	if err != nil {
		fmt.Printf("    Setup error: %v\n", err)
		return pt
	}
	pt.Setup = time.Since(start)
	fmt.Printf("    Setup:       %v\n", pt.Setup)

	var pkBuf, vkBuf bytes.Buffer
	if _, err := pk.WriteTo(&pkBuf); err != nil {
		fmt.Printf("    PK WriteTo error: %v\n", err)
		return pt
	}
	if _, err := vk.WriteTo(&vkBuf); err != nil {
		fmt.Printf("    VK WriteTo error: %v\n", err)
		return pt
	}
	pt.PKBytes = pkBuf.Len()
	pt.VKBytes = vkBuf.Len()
	fmt.Printf("    PK size:     %s compressed\n", humanBytes(pt.PKBytes))
	fmt.Printf("    VK size:     %s compressed\n", humanBytes(pt.VKBytes))

	witness, err := frontend.NewWitness(assignment, ecc.BN254.ScalarField())
	if err != nil {
		fmt.Printf("    Witness error: %v\n", err)
		return pt
	}
	publicWitness, err := witness.Public()
	if err != nil {
		fmt.Printf("    Public witness error: %v\n", err)
		return pt
	}

	start = time.Now()
	proof, err := groth16.Prove(ccs, pk, witness)
	if err != nil {
		fmt.Printf("    Prove error: %v\n", err)
		return pt
	}
	pt.Prove = time.Since(start)
	fmt.Printf("    Prove:       %v\n", pt.Prove)

	var compressed bytes.Buffer
	if _, err := proof.WriteTo(&compressed); err != nil {
		fmt.Printf("    WriteTo error: %v\n", err)
		return pt
	}
	pt.ProofBytes = compressed.Len()
	fmt.Printf("    Proof size:  %d B compressed\n", pt.ProofBytes)

	start = time.Now()
	if err := groth16.Verify(proof, vk, publicWitness); err != nil {
		fmt.Printf("    Verify error: %v\n", err)
		return pt
	}
	pt.Verify = time.Since(start)
	fmt.Printf("    Verify:      %v\n", pt.Verify)
	return pt
}

func printSummary(rows []rowResult) {
	fmt.Println("=== Summary ===")
	fmt.Println()
	fmt.Printf("%-10s %-14s %-10s %-10s %-10s %-10s %-10s %-10s\n", "Shape", "Circuit", "Constr", "Compile", "Prove", "Verify", "PK", "VK")
	fmt.Printf("%-10s %-14s %-10s %-10s %-10s %-10s %-10s %-10s\n", "-----", "-------", "------", "-------", "-----", "------", "--", "--")
	for _, r := range rows {
		shape := fmt.Sprintf("%d/%d", r.N, r.M)
		fmt.Printf("%-10s %-14s %-10d %-10s %-10s %-10s %-10s %-10s\n",
			shape, "utxo_circuit", r.Utxo.Constraints, short(r.Utxo.Compile), short(r.Utxo.Prove), short(r.Utxo.Verify),
			humanBytes(r.Utxo.PKBytes), humanBytes(r.Utxo.VKBytes))
		fmt.Printf("%-10s %-14s %-10d %-10s %-10s %-10s %-10s %-10s\n",
			"", "tree_circuit", r.Tree.Constraints, short(r.Tree.Compile), short(r.Tree.Prove), short(r.Tree.Verify),
			humanBytes(r.Tree.PKBytes), humanBytes(r.Tree.VKBytes))
	}
	fmt.Println()
}

func humanBytes(n int) string {
	switch {
	case n >= 1<<30:
		return fmt.Sprintf("%.2f GiB", float64(n)/float64(1<<30))
	case n >= 1<<20:
		return fmt.Sprintf("%.1f MiB", float64(n)/float64(1<<20))
	case n >= 1<<10:
		return fmt.Sprintf("%.1f KiB", float64(n)/float64(1<<10))
	default:
		return fmt.Sprintf("%d B", n)
	}
}

func short(d time.Duration) string {
	switch {
	case d >= time.Second:
		return fmt.Sprintf("%.2fs", d.Seconds())
	case d >= time.Millisecond:
		return fmt.Sprintf("%dms", d.Milliseconds())
	default:
		return fmt.Sprintf("%dµs", d.Microseconds())
	}
}
