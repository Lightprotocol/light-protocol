import ProvenZK
import FormalVerification.Circuit
import FormalVerification.Merkle

theorem foo := NonInclusionCircuit_correct

def main : IO Unit := IO.println "verified"
