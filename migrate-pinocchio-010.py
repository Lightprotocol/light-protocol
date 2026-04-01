#!/usr/bin/env python3
"""
Pinocchio 0.9 → 0.10 migration script.
Only modifies pinocchio-specific code. Does NOT touch non-pinocchio types.
"""
import os
import re

def process_file(path, is_pinocchio_program=False):
    """Process a single .rs file for pinocchio 0.10 migration."""
    with open(path) as f:
        content = f.read()

    if 'pinocchio' not in content:
        return False

    original = content

    # =========================================================
    # 1. Fix FULL pinocchio:: paths (safe - only matches pinocchio prefix)
    # =========================================================
    content = content.replace('pinocchio::program_error::ProgramError', 'pinocchio::error::ProgramError')
    content = content.replace('pinocchio::program_error', 'pinocchio::error')
    content = content.replace('pinocchio::account_info::AccountInfo', 'pinocchio::AccountView')
    content = content.replace('pinocchio::account_info', 'pinocchio::account')
    content = content.replace('pinocchio::pubkey::Pubkey', 'pinocchio::address::Address')
    content = content.replace('pinocchio::pubkey', 'pinocchio::address')
    content = content.replace('pinocchio::cpi::slice_invoke_signed', 'pinocchio::cpi::invoke_signed_with_slice')
    content = content.replace('pinocchio::cpi::slice_invoke', 'pinocchio::cpi::invoke_with_slice')

    # =========================================================
    # 2. Fix compound imports: `use pinocchio::{...}`
    # =========================================================
    def fix_pinocchio_import(match):
        block = match.group(0)
        # Fix module paths inside pinocchio::{...}
        block = block.replace('account_info::AccountInfo', 'AccountView')
        block = block.replace('program_error::ProgramError', 'error::ProgramError')
        block = block.replace('pubkey::Pubkey', 'address::Address')
        block = block.replace('instruction::Seed', 'cpi::Seed')
        block = block.replace('instruction::Signer', 'cpi::Signer')
        block = block.replace('instruction::AccountMeta', 'instruction::InstructionAccount')
        block = block.replace('instruction::Instruction,', 'instruction::InstructionView,')
        block = block.replace('instruction::Instruction}', 'instruction::InstructionView}')
        block = block.replace('instruction::Account,', '')
        block = block.replace('instruction::Account}', '}')
        # Remove msg from pinocchio imports (moved to solana_msg)
        block = re.sub(r',\s*msg\b', '', block)
        block = re.sub(r'\bmsg,\s*', '', block)
        block = re.sub(r'\bmsg\}', '}', block)
        return block

    content = re.sub(r'use pinocchio::\{[^}]+\}', fix_pinocchio_import, content)

    # =========================================================
    # 3. Add solana_msg import if msg! is used but not imported
    # =========================================================
    if 'msg!' in content and 'solana_msg' not in content:
        if 'use pinocchio' in content:
            content = re.sub(
                r'(use pinocchio::\{[^}]+\};)',
                r'\1\nuse solana_msg::msg;',
                content, count=1
            )

    # =========================================================
    # 4. Remove #[cfg(feature = "pinocchio")] From<ProgramError> impls
    #    (pinocchio::error::ProgramError IS solana_program_error::ProgramError now)
    # =========================================================
    lines = content.split('\n')
    new_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if '#[cfg(feature = "pinocchio")]' == line.strip() and i + 1 < len(lines):
            next_line = lines[i + 1].strip()
            if 'pinocchio' in next_line and 'ProgramError' in next_line:
                # Skip the entire impl block
                i += 1
                depth = 0
                while i < len(lines):
                    if '{' in lines[i]: depth += lines[i].count('{')
                    if '}' in lines[i]: depth -= lines[i].count('}')
                    i += 1
                    if depth <= 0: break
                continue
        new_lines.append(line)
        i += 1
    content = '\n'.join(new_lines)

    # =========================================================
    # 5. For pinocchio PROGRAMS: rename methods on pinocchio types
    #    Only do this in files that are part of pinocchio program crates
    # =========================================================
    if is_pinocchio_program:
        # These method renames only apply to pinocchio's AccountView type
        # NOT to the AccountInfoTrait (which keeps the old names)
        pass  # Don't do global renames - handle per-crate

    if content != original:
        with open(path, 'w') as f:
            f.write(content)
        return True
    return False


# =========================================================
# Main: Process all files
# =========================================================
count = 0
for root, dirs, files in os.walk('.'):
    if any(x in root for x in ['external/', 'target/', '.local/', '.git/']):
        dirs[:] = []
        continue
    for f in files:
        if f.endswith('.rs'):
            path = os.path.join(root, f)
            if process_file(path):
                count += 1

print(f"Processed {count} files (paths + imports + From impls)")
