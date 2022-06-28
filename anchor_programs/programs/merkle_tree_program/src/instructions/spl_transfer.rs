use anchor_lang::prelude::*;

/*
pub fn token_transfer<'a, 'b>(
    token_program: &'b AccountInfo<'a>,
    source: &'b AccountInfo<'a>,
    destination: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    seed: &[u8],
    bump_seed: &[u8],
    amount: u64,
) -> Result<(), ProgramError> {
    let authority_signature_seeds = [seed, bump_seed];

    let signers = &[&authority_signature_seeds[..]];

    let ix = spl_token::instruction::transfer(
        token_program.key,
        source.key,
        destination.key,
        authority.key,
        &[],
        amount,
    )?;
    invoke_signed(
        &ix,
        &[
            source.clone(),
            destination.clone(),
            authority.clone(),
            token_program.clone(),
        ],
        signers,
    )?;
    Ok(())
}
*/
