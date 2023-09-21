import { Idl } from "@coral-xyz/anchor";
import { IdlAccountDef, IdlAccountItem } from "@coral-xyz/anchor/dist/cjs/idl";

export type zkIdl = Idl;

// Recursive generic type parser till dimension = 4 i.e. for
// { array: [{ array: [{ array: [{ array: ["u8", 2] }, 3] }, 4] }, 5] } => string[][][][]

type ConvertArray<T> = T extends { array: infer U }
  ? U extends [{ array: infer K }, number]
    ? Array<ConvertArray<{ array: K }>>
    : U extends ["u8", number]
    ? Array<string>
    : never
  : T extends "u8"
  ? string
  : never;

// Generic type parser according to the parsed public inputs for a solana program instruction i.e. for
// { array: [{ array: [{ array: [{ array: ["u8", 2] }, 3] }, 4] }, 5] } => number[][][][][]
type ConvertToParsedArray<T> = T extends { array: infer U }
  ? U extends [{ array: infer K }, number]
    ? Array<ConvertArray<{ array: K }>>
    : U extends ["u8", number]
    ? Array<Array<number>>
    : never
  : T extends "u8"
  ? Array<number>
  : never;

// create a mapped type combing name as key and type as property
type MapObjectKeys<T extends Array<{ name: any; type: any }>> = {
  [Key in T[number]["name"]]: Extract<T[number], { name: Key }>["type"];
};

// Parse Rust types into TS: used for proof inputs
type CircuitInputsObject<T> = {
  [Property in keyof T]: ConvertArray<T[Property]>;
};

// Parse Rust types into TS: used for public inputs
type CircuitParsedPubInObject<T> = {
  [Property in keyof T]: ConvertToParsedArray<T[Property]>;
};

type ExtractPrefix<T extends string> = T extends
  | `zK${infer P}ProofInputs`
  | `zK${infer P}PublicInputs`
  ? P
  : never;

// Extract unique circuit names from VerifierIdls Union Type
export type CircuitNames = ExtractPrefix<ZKAccounts["name"]>;

// Optional type: circuit proof inputs object by selecting full zk account name from the idl
type ZKProofInputsObjectFullName<
  Idl extends zkIdl,
  AccountName extends ZKAccounts["name"],
> = CircuitInputsObject<MapObjectKeys<SelectZKAccount<Idl, AccountName>>>;

export type ProofInputs<
  Idl extends zkIdl,
  CircuitName extends CircuitNames,
> = CircuitInputsObject<
  MapObjectKeys<SelectZKAccount<Idl, `zK${CircuitName}ProofInputs`>>
>;

export type ParsedPublicInputs<
  Idl extends zkIdl,
  CircuitName extends CircuitNames,
> = CircuitParsedPubInObject<
  MapObjectKeys<SelectZKAccount<Idl, `zK${CircuitName}PublicInputs`>>
>;

type Account = IdlAccountDef;

type SelectAccount<
  AccountName extends string,
  T extends Account[] | undefined,
> = T extends [infer First, ...infer Rest]
  ? First extends { name: AccountName }
    ? First
    : SelectAccount<AccountName, Rest extends Account[] ? Rest : never>
  : never;

// Select a specific zk account among ZKAccounts type
type SelectZKAccount<
  Idl extends zkIdl,
  AccountName extends ZKAccounts["name"],
> = SelectAccount<AccountName, Idl["accounts"]> extends {
  name: any;
  type: { kind: any; fields: any };
}
  ? SelectAccount<AccountName, Idl["accounts"]>["type"]["fields"]
  : never;

type Accounts<Idl extends zkIdl> = {
  [N in Idl["name"]]: Account;
};
type ZKAccounts = FetchZKAccounts<Accounts<zkIdl>>;

// Filter circuit zk accounts
type FetchZKAccounts<T> = T extends Accounts<zkIdl>
  ? T["name"] extends `zK${infer _}`
    ? T
    : never
  : never;

type ZKProofAccounts = FetchProofAccounts<ZKAccounts>;
type FetchProofAccounts<T> = T extends ZKAccounts
  ? T["name"] extends `${infer _}Proof${infer _}`
    ? T
    : never
  : never;

type SelectZKPubInAccount<
  Idl extends zkIdl,
  CircuitName extends CircuitNames,
> = SelectZKAccount<Idl, `zK${CircuitName}PublicInputs`>;

type ZKPublicInAccounts = FetchPublicInAccounts<ZKAccounts>;
type FetchPublicInAccounts<T> = T extends ZKAccounts
  ? T["name"] extends `${infer _}Public${infer _}`
    ? T
    : never
  : never;
