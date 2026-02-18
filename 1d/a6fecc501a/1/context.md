# Session Context

## User Prompts

### Prompt 1

I want to formally verifiy light-account sdk-libs/account see this for context /Users/ananas/dev/experiments/formally-verified-programs
use init kani skill 
give me a list of 10 things that make sense to formally verify

### Prompt 2

**Initializing Kani Formal Verification Context**

This command loads comprehensive documentation of Kani usage from multiple production codebases, including Firecracker (35 harnesses), zerocopy (6 harnesses), and Otter Verify framework. Learn patterns, techniques, and best practices for formal verification in Rust and Solana programs.

## Required Reading Sequence

**1. Read Main Index and Navigation Guide**

```bash
cat /Users/ananas/dev/claude-context/kani/CLAUDE.md
```
*Complete repository s...

### Prompt 3

the most important part is this /Users/ananas/dev/light-protocol3/sdk-libs/sdk-types/src/interface/program

### Prompt 4

give me a list with invariants for each

### Prompt 5

ok plan this out, use skill mode auto and skill plan reviews
this should be an incremantal step by step long running plan likely will take multiple hours

### Prompt 6

# Autonomous Execution Mode

You are now in **autonomous execution mode**. This mode is optimized for long-running tasks (hours) that should complete without user intervention.

## Core Principles

1. **NEVER STOP** until the goal is reached
2. **NEVER ASK** questions during execution (ask everything in planning phase)
3. **NEVER REQUEST** new permissions - work within what's allowed
4. **ALWAYS RECOVER** from errors autonomously
5. **USE SUBAGENTS** when stuck or for parallel work

## Instructi...

### Prompt 7

Add these two sections explicitly to the current plan:

## 1. Review Tasks

Add 5 review tasks to the plan. Each review should:
- Use a subagent with the review skill to check whether the goal was achieved
- If the goal was NOT achieved, use a subagent to assess the review and plan fixes in line with the original plan's goal
- Use another subagent to implement the fixes

## 2. Bug/Issue Handling Loop

In case you encounter any bugs or issues:
1. Use an agent to investigate and plan how to fix th...

### Prompt 8

[Request interrupted by user for tool use]

### Prompt 9

Use a subagent with model=opus to validate the current plan.

The subagent should analyze the plan and answer these questions:

1. Are there any open questions?
2. Are there any conflicting objectives?

Report findings clearly and suggest resolutions if issues are found.


ARGUMENTS: does the split up make sense? are any phases too complex and should be divided into more steps?

### Prompt 10

do it

### Prompt 11

This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.

Analysis:
Let me go through the conversation chronologically to capture all important details.

1. **Initial Request**: User wants to formally verify `sdk-libs/account` (light-account) using Kani. They reference `/Users/ananas/dev/experiments/formally-verified-programs` for context and want to use the "init kani" skill. They want a list of 10 th...

### Prompt 12

ok what about a full harness now?

### Prompt 13

[Request interrupted by user for tool use]

### Prompt 14

what do you prove exactly now?

### Prompt 15

compression must prove:
1. every input account that is compressible creates a compressed account with the correct data, discriminator etc

### Prompt 16

what values are symbolic?

### Prompt 17

so the custom account data is  a u64?

### Prompt 18

182  

⏺ Summary:                            

  Yes, kani::vec::exact_vec is a 3x speedup over manual push loops and should be the default way to create fixed-length symbolic Vecs. It works by allocating one Box<[T; N]> with 
  a single kani::any() then converting to Vec, avoiding N incremental push + realloc reasoning steps.
                                                                                                                                                                         ...

### Prompt 19

see program formal verification repo

### Prompt 20

no

### Prompt 21

[Request interrupted by user]

### Prompt 22

kani::vec::exact_vec is more efficient
use it

### Prompt 23

This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.

Analysis:
Let me chronologically analyze the conversation:

1. **Context from previous session**: The conversation is a continuation from a prior session. There were already 28 passing Kani harnesses. A 19-step plan existed at `/Users/ananas/.claude/plans/swirling-popping-sunset.md` for verifying the compress processor incrementally. Steps 1-9 w...

### Prompt 24

<task-notification>
<task-id>b9becbf</task-id>
<output-file>/private/tmp/claude-501/-Users-ananas-dev-light-protocol3/tasks/b9becbf.output</output-file>
<status>completed</status>
<summary>Background command "Run verify_compression_arbitrary_data with 8 symbolic bytes" completed (exit code 0)</summary>
</task-notification>
Read the output file to retrieve the result: /private/tmp/claude-501/-Users-ananas-dev-light-protocol3/tasks/b9becbf.output

### Prompt 25

<task-notification>
<task-id>b67affb</task-id>
<output-file>REDACTED.output</output-file>
<status>completed</status>
<summary>Background command "Run full Kani suite" completed (exit code 0)</summary>
</task-notification>
Read the output file to retrieve the result: REDACTED.output

### Prompt 26

summarize the current state for me

### Prompt 27

critically check with one subagent for each that these verifications actually verify what they say they do  What's Verified

  Compression pipeline (end-to-end via dispatch_compress_pda_accounts):
  - verify_compression_arbitrary_data -- 8 fully symbolic bytes through the full dispatch pipeline. Proves: correct discriminators, canonical output data, zeroed hash first byte,
  address derivation, lamport values.
  - verify_every_compressible_account_produces_correct_output -- 2 PDAs with independe...

### Prompt 28

we should approach this in a more methodical way,
what do we need to prove to verify that statechanges are correct?
how can we prove that efficiently?
did we actually prove what we set out to do?

### Prompt 29

- P2: output.data_hash == hash(output.data) with [0]=0
can we stub the hash function so to all 1s with one leading 0 so that we can prove that the state was always hashed with that specific function?

### Prompt 30

This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.

Analysis:
Let me chronologically analyze the conversation:

1. **Session start**: This is a continuation from a previous session. The context summary tells us:
   - 42 Kani harnesses were passing
   - A 19-step plan existed for verifying the compress processor
   - Steps 1-17 were completed in the prior session
   - Key production code changes: ...

### Prompt 31

[Request interrupted by user]

### Prompt 32

or can we do sth with lazy static to increment a number?

### Prompt 33

[Request interrupted by user for tool use]

### Prompt 34

what about the lazy static idea?

### Prompt 35

[Request interrupted by user]

### Prompt 36

Pubkey::new_unique() does sth like that
check it out solana-pubkey in .cargo/registry

### Prompt 37

nice ok next how do we prove the address relationship?

### Prompt 38

[Request interrupted by user for tool use]

### Prompt 39

lets brainstorm first use 5 agents to give different ideas

### Prompt 40

hm in this case we can just have the pubkey as seed so just zeroing the first byte of the seed would mark it

### Prompt 41

[Request interrupted by user]

### Prompt 42

OK

### Prompt 43

what other issues does the formal verification have?

### Prompt 44

2. Address stub ignores address_space and program_id (P3 is partial)
  We prove the PDA key was used as the seed, but NOT that the correct address_space or program_id were passed. If the code used address_space[1] instead of [0], or a wrong
  program_id, the stub wouldn't catch it.

good point how can we include this?

