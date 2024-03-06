/// actually , we should probably just do the hlper for compilation, otherwise
///folks will have to set it manually. / i.e getRecentValidityProof()
///addValidityProofToCompiledInstruction() otherwise: compileInstruction(ix,
///validityProof)
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

//@ts-ignore
if (import.meta.vitest) {
  //@ts-ignore
  const { it, expect, describe } = import.meta.vitest;

  describe("Validity Proof Functions", () => {
    describe("placeholderValidityProof", () => {
      it("should create a validity proof with correct shape", () => {
        const validityProof = placeholderValidityProof();
        expect(validityProof.proofA.length).toBe(32);
        expect(validityProof.proofB.length).toBe(64);
        expect(validityProof.proofC.length).toBe(32);
      });
    });

    describe("checkValidityProofShape", () => {
      it("should not throw an error for valid proof shape", () => {
        const validProof = {
          proofA: new Uint8Array(32),
          proofB: new Uint8Array(64),
          proofC: new Uint8Array(32),
        };
        expect(() => checkValidityProofShape(validProof)).not.toThrow();
      });

      it("should throw an error for an invalid proof", () => {
        const invalidProof = {
          proofA: new Uint8Array(31), // incorrect length
          proofB: new Uint8Array(64),
          proofC: new Uint8Array(32),
        };
        expect(() => checkValidityProofShape(invalidProof)).toThrow(
          "ValidityProof has invalid shape"
        );
      });
    });
  });
}
