# Session Context

## User Prompts

### Prompt 1

check the current diff what do we currently prove for which functions?

### Prompt 2

which of these harnesses really make sense?

### Prompt 3

remove all of these   ~7 are weak -- they test trivial constructors or mocks rather than production code.

  ~9 are unit tests dressed up as Kani proofs. They'd be better as #[test] functions -- they don't benefit from symbolic execution because they test constants, trivial setters, or
   stub behavior. ~7 are modeling proofs that verify logic the harness itself wrote, not the actual codebase. These give false confidence. The decompression module is the worst offender -- every
  proof there eith...

### Prompt 4

ok what remains in compression?

### Prompt 5

what are the inputs and outputs to dispatch_compress_pda_accounts ?

### Prompt 6

ok read  /Users/ananas/dev/experiments/formally-verified-programs/GUIDE.md

### Prompt 7

I want to write a contract as outlined in the guide for the dispatch function

### Prompt 8

[Request interrupted by user for tool use]

### Prompt 9

we are brainstorming now describe to me in a numbered list what you want to prove for what

### Prompt 10

ok this is insufficient, we need to prove relation ship between inputs and outputs

### Prompt 11

and we need to prove that the correct accounts are skipped

### Prompt 12

and not skipped

### Prompt 13

ok what else do we need to prove to prove complete correctness?

### Prompt 14

ok so split it up what for accounts, for output data, for other variables create a numbered list each

### Prompt 15

ok write this into your plan

### Prompt 16

[Request interrupted by user for tool use]

### Prompt 17

Use a subagent with model=opus to validate the current plan.

The subagent should analyze the plan and answer these questions:

1. Are there any open questions?
2. Are there any conflicting objectives?

Report findings clearly and suggest resolutions if issues are found.

### Prompt 18

ok can you write up the properties in a dispact_compress.md file where would be the best place to put it?

### Prompt 19

we use old in /Users/ananas/dev/experiments/formally-verified-programs and you can ask deepwiki

### Prompt 20

[Request interrupted by user for tool use]

### Prompt 21

what harnesses cover what?

### Prompt 22

can we stub the output creation?

### Prompt 23

I dont follow on these - canonicalize_for_hashing -- returns Ok(vec![]) (never inspected in contract)
  - hash_account_data -- returns Ok([0u8; 32]) (never inspected in contract)
  - Sha256BE::hash -- returns Ok([0u8; 32]) (never inspected in contract)

### Prompt 24

├── canonicalize_for_hashing(account_data)  → data_bytes                                                                                                                       
    ├── hash_account_data(&data_bytes)          → output_data_hash                                                                                                                 
    ├── Sha256BE::hash(&account_key)            → input_data_hash    we should do a separate input accounts proof first ...

### Prompt 25

[Request interrupted by user for tool use]

### Prompt 26

walk me through your composition it is ok to refactor the code so that it is easier to compose

### Prompt 27

this  is layer 2   Layer: 1c
  Function: build_compression_output
  Contract Proves: B2-B8, C4-C6, D1: field passthrough, discriminators, lamports
  Stubs: none

### Prompt 28

and layer 2 should also have prepare_account_for_compression right?

### Prompt 29

build_compression_output also stubs stuff

### Prompt 30

layer 1 should be stubs for layer 2 nothgin else

### Prompt 31

[Request interrupted by user]

### Prompt 32

thats why I dont see build_compression_output this as layer1

### Prompt 33

[Request interrupted by user]

### Prompt 34

<local-command-stderr>Error: Compaction canceled.</local-command-stderr>

### Prompt 35

This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.

Analysis:
Let me go through the conversation chronologically:

1. User asks to check the current diff and what's being formally verified
2. I examine the diff (refactoring pda.rs and processor.rs) and all Kani proof files
3. I provide a comprehensive summary of all proofs across 6 modules
4. User asks which harnesses "really make sense"
5. I cat...

### Prompt 36

ok whats the current state?

### Prompt 37

we are still disussing

### Prompt 38

I want a modify input account function with a contract similar to build_compression_output

### Prompt 39

├── Task 1: hash_account_data         → C3                                                                                                                                     
    ├── Task 2: canonicalize_for_hashing  → C1, E1     these should only be used in modify_input_account

### Prompt 40

hm cant we just make this a single stub

### Prompt 41

ok what else is requried to verify dispatch compress completely?

### Prompt 42

ok no skip it

### Prompt 43

ok show me the complete plan

### Prompt 44

[Request interrupted by user for tool use]

### Prompt 45

Use a subagent with model=opus to validate the current plan.

The subagent should analyze the plan and answer these questions:

1. Are there any open questions?
2. Are there any conflicting objectives?

Report findings clearly and suggest resolutions if issues are found.

### Prompt 46

use the existing stub_sha256_hash stub

### Prompt 47

[Request interrupted by user for tool use]

### Prompt 48

use mode auto skill

### Prompt 49

# Autonomous Execution Mode

You are now in **autonomous execution mode**. This mode is optimized for long-running tasks (hours) that should complete without user intervention.

## Core Principles

1. **NEVER STOP** until the goal is reached
2. **NEVER ASK** questions during execution (ask everything in planning phase)
3. **NEVER REQUEST** new permissions - work within what's allowed
4. **ALWAYS RECOVER** from errors autonomously
5. **USE SUBAGENTS** when stuck or for parallel work

## Instructi...

### Prompt 50

[Request interrupted by user for tool use]

