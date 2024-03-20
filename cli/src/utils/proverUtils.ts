export type ProofInputs = {
  root: string[];
  inPathIndices: number[];
  inPathElements: string[][];
  leaf: string[];
};
export function provingArgs(inputs: string): string {
  const arg0 = "echo";
  const arg1 = inputs;
  const arg2 = "./bin/light-prover";
  const arg3 = "prove";

  const arg4 = provingKey(parseProofInputs(inputs).root.length);
  const args = [arg0, "'", arg1, "' | ", arg2, arg3, arg4].join(" ");
  return args;
}

export function verifyingArgs(
  proof: string,
  roots: string[],
  leafs: string[],
): string {
  const arg0 = "echo";
  const arg1 = proof;
  const arg2 = "./bin/light-prover";
  const arg3 = "verify";
  const arg4 = provingKey(roots.length);
  const arg5 = `--roots ${roots}`;
  const arg6 = `--leafs ${leafs}`;

  const args = [arg0, "'", arg1, "' | ", arg2, arg3, arg4, arg5, arg6].join(
    " ",
  );

  return args;
}

function provingKey(utxos: number, height: number = 26): string {
  return `-k ./bin/circuits/inclusion_${height}_${utxos}.key`;
}

function parseProofInputs(json: string): ProofInputs {
  const inputs: ProofInputs = JSON.parse(json);
  return inputs;
}
