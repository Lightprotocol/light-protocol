/// actually , we should probably just do the hlper for compilation, otherwise folks will have to set it manually.

//// i.e getRecentValidityProof()

/// addValidityProofToCompiledInstruction()

/// otherwise: compileInstruction(ix, validityProof)
export type ValidityProof = {
  proofA: Uint8Array;
  proofB: Uint8Array;
  proofC: Uint8Array;
};

export const placeholderValidityProof = () => ({
  proofA: new Uint8Array(Array.from({ length: 32 }, (_, i) => i + 1)),
  proofB: new Uint8Array(Array.from({ length: 64 }, (_, i) => i + 1)),
  proofC: new Uint8Array(Array.from({ length: 32 }, (_, i) => i + 1)),
});

export const checkValidityProofShape = (proof: ValidityProof) => {
  if (
    proof.proofA.length !== 32 ||
    proof.proofB.length !== 64 ||
    proof.proofC.length !== 32
  ) {
    throw new Error("ValidityProof has invalid shape");
  }
};
