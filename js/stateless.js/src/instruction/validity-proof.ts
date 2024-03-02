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
  proofA: new Uint8Array(32),
  proofB: new Uint8Array(64),
  proofC: new Uint8Array(32),
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
