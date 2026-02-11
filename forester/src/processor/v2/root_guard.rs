#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootReconcileDecision {
    Proceed,
    WaitForIndexer,
    ResetToOnchainAndProceed([u8; 32]),
    ResetToOnchainAndStop([u8; 32]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentDecision {
    /// No overlap; safe to process the batch.
    Process,
    /// Batch overlaps already-processed items; skip it.
    SkipOverlap,
    /// There's a gap between what's expected and where this batch starts.
    Gap,
    /// Local staging tree is stale relative to the indexer snapshot; invalidate state.
    StaleTree,
}

/// Decide how to reconcile roots after fetching an indexer snapshot root and an on-chain root.
///
/// Inputs:
/// - `expected_root`: the processor's local expected root (may be zero/uninitialized)
/// - `indexer_root`: the indexer's snapshot root for the fetched queue data
/// - `onchain_root`: the authoritative on-chain root
pub fn reconcile_roots(
    expected_root: [u8; 32],
    indexer_root: [u8; 32],
    onchain_root: [u8; 32],
) -> RootReconcileDecision {
    if expected_root == [0u8; 32] {
        // Uninitialized expected root — proceed but adopt the indexer root.
        // Validate that indexer and on-chain agree when possible.
        if indexer_root != onchain_root {
            tracing::warn!(
                "Proceeding with uninitialized expected root, but indexer root ({:?}) != onchain root ({:?}). Indexer may be stale.",
                &indexer_root[..4],
                &onchain_root[..4],
            );
        }
        return RootReconcileDecision::Proceed;
    }
    if indexer_root == expected_root {
        return RootReconcileDecision::Proceed;
    }

    if onchain_root == expected_root {
        return RootReconcileDecision::WaitForIndexer;
    }

    if indexer_root == onchain_root {
        return RootReconcileDecision::ResetToOnchainAndProceed(onchain_root);
    }

    RootReconcileDecision::ResetToOnchainAndStop(onchain_root)
}

/// Decide whether a particular batch should be processed given:
/// - where the indexer snapshot starts (`data_start_index`)
/// - where the staging tree currently is (`tree_next_index`)
/// - the batch start offset within the snapshot (`start`)
///
/// The return value is intentionally coarse-grained so callers can decide whether to retry,
/// invalidate caches, or simply skip work.
pub fn reconcile_alignment(
    tree_next_index: usize,
    data_start_index: usize,
    start: usize,
) -> AlignmentDecision {
    if data_start_index > tree_next_index {
        return AlignmentDecision::StaleTree;
    }

    let absolute_index = data_start_index + start;

    if absolute_index < tree_next_index {
        return AlignmentDecision::SkipOverlap;
    }
    if absolute_index > tree_next_index {
        return AlignmentDecision::Gap;
    }

    AlignmentDecision::Process
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn proceeds_when_expected_is_zero() {
        assert_eq!(
            reconcile_roots(root(0), root(1), root(2)),
            RootReconcileDecision::Proceed
        );
    }

    #[test]
    fn proceeds_when_expected_matches_indexer() {
        assert_eq!(
            reconcile_roots(root(9), root(9), root(8)),
            RootReconcileDecision::Proceed
        );
    }

    #[test]
    fn waits_when_onchain_confirms_expected() {
        assert_eq!(
            reconcile_roots(root(7), root(6), root(7)),
            RootReconcileDecision::WaitForIndexer
        );
    }

    #[test]
    fn resets_and_proceeds_when_indexer_matches_onchain() {
        assert_eq!(
            reconcile_roots(root(7), root(6), root(6)),
            RootReconcileDecision::ResetToOnchainAndProceed(root(6))
        );
    }

    #[test]
    fn resets_and_stops_on_three_way_divergence() {
        assert_eq!(
            reconcile_roots(root(7), root(6), root(5)),
            RootReconcileDecision::ResetToOnchainAndStop(root(5))
        );
    }

    #[test]
    fn alignment_stale_when_data_starts_after_tree() {
        assert_eq!(reconcile_alignment(10, 11, 0), AlignmentDecision::StaleTree);
    }

    #[test]
    fn alignment_skips_full_overlap() {
        assert_eq!(
            reconcile_alignment(10, 0, 0),
            AlignmentDecision::SkipOverlap
        );
    }

    #[test]
    fn alignment_skips_partial_overlap() {
        assert_eq!(
            reconcile_alignment(10, 0, 8),
            AlignmentDecision::SkipOverlap
        );
    }

    #[test]
    fn alignment_processes_when_no_overlap() {
        assert_eq!(reconcile_alignment(10, 0, 10), AlignmentDecision::Process);
    }

    #[test]
    fn alignment_reports_gap_when_batch_starts_after_expected() {
        assert_eq!(reconcile_alignment(10, 0, 12), AlignmentDecision::Gap);
    }
}
