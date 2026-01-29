<!-- cargo-rdme start -->

Derive macros for InstructionDecoder implementations

This crate provides two macros:
1. `#[derive(InstructionDecoder)]` - For instruction enums (native programs)
2. `#[instruction_decoder]` - Attribute macro for Anchor program modules

The attribute macro extracts function names from the program module and generates
an instruction enum with `#[derive(InstructionDecoder)]` applied.

## Enhanced InstructionDecoder for Anchor Programs

The derive macro supports an enhanced mode that references Anchor-generated types
for account names and parameter decoding:

```rust
use light_instruction_decoder_derive::InstructionDecoder;

#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "MyProgram111111111111111111111111111111111",
    program_name = "My Program"
)]
pub enum MyInstruction {
    #[instruction_decoder(accounts = CreateRecord, params = CreateRecordParams)]
    CreateRecord,

    #[instruction_decoder(accounts = UpdateRecord)]
    UpdateRecord,
}
```

This generates a decoder that:
- Gets account names from `<AccountsType<'_>>::ACCOUNT_NAMES`
- Decodes instruction data using `ParamsType::try_from_slice()` with Debug output

<!-- cargo-rdme end -->
