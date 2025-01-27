use crate::{context::AcpAccount, errors::AccountCompressionErrorCode};

pub fn get_queue_and_tree_accounts<'a, 'b, 'info>(
    accounts: &'b mut [AcpAccount<'a, 'info>],
    queue_index: usize,
    tree_index: usize,
) -> std::result::Result<
    (&'b mut AcpAccount<'a, 'info>, &'b mut AcpAccount<'a, 'info>),
    AccountCompressionErrorCode,
> {
    let (smaller, bigger) = if queue_index < tree_index {
        (queue_index, tree_index)
    } else {
        (tree_index, queue_index)
    };
    let (left, right) = accounts.split_at_mut(bigger);
    let smaller_ref = &mut left[smaller];
    let bigger_ref = &mut right[0];
    Ok(if queue_index < tree_index {
        (smaller_ref, bigger_ref)
    } else {
        (bigger_ref, smaller_ref)
    })
}
