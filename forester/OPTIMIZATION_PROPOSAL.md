# Forester Zero-Latency Optimization Proposal

**Date**: 2025-02-15
**Status**: In progress
**Goal**: Reduce forester transaction submission latency from 10-60s to <1s

---

## Executive Summary

Forester V2 regained stability after re-introducing persistent staging caches and enforcing sequential root chaining, and we have since consolidated every per-tree concern inside `StateTreeCoordinator`. The coordinator now owns the speculative warm-up engine, pending queue tracking, and staging snapshots, while `EpochManager` merely routes queue updates and slot events. The speculative path can already build batches and proofs ahead of time, but invalidation, triggering, and observability remain immature. Completing those guardrails—and cleaning up the surrounding architecture—will unlock the promised <1 s submission latency without sacrificing correctness.

**Expected Impact**: When speculation is triggered at the right time and invalidated aggressively, we eliminate 10-60 s from the critical path, keep caches hot, and slash noise from redundant logging/state tracking.

## Current Architecture Snapshot (Feb 2025)

- **Coordinator Ownership**: Each tree gets a dedicated `StateTreeCoordinator` (spawned once per account and kept for the entire run). The coordinator encapsulates staging cache, queue deltas, speculative engine, and execution metrics.
- **Speculative Engine**: `SpeculativeEngine` stores queue updates, an inflight flag, and a prepared `SpeculativeJob` (pattern, proofs, staging snapshot, metrics). `prepare_speculative_job()` builds everything ahead of time; `try_execute_speculative_job()` is invoked whenever we suspect the slot is live.
- **Shared Queue Router**: Work queues remain global (`work_queue::Route`), and the epoch manager only forwards queue updates/slot ticks to the owning coordinator; we no longer expose controller internals.
- **Indexer Interactions**: `wait_for_indexer` is called only when the code is about to hit Photon (REST or gRPC) to make sure the service is in sync with RPC; normal slot processing uses cached data.
- **Open Questions**: Whether caches, staging trees, and `TreeState` should remain distinct structs and whether we should keep a `StateTreeCoordinator` alive for every tree indefinitely (current lean: yes, paired with the epoch manager lifecycle).

---

## System Architecture Analysis

### Current Information Flow

```
1. USER TRANSACTION
   └─> Solana: Appends items to OutputQueue/InputQueue
       └─> Emits program logs

2. PHOTON INDEXER (Real-time)
   ├─> Ingester: Monitors Solana blocks via Geyser or RPC
   ├─> Parser: Parses txs → StateUpdate
   ├─> Persister: Writes to PostgreSQL
   └─> EventPublisher: IMMEDIATELY emits IngestionEvent
       ├─ OutputQueueInsert
       ├─ NullifierQueueInsert
       └─ AddressQueueInsert

3. PHOTON gRPC SERVICE
   ├─> GrpcEventSubscriber: Receives IngestionEvents → QueueUpdate
   ├─> QueueMonitor: Polls DB every N ms (fallback)
   └─> Streams QueueUpdate to forester (PUSH model)

4. FORESTER (Current Implementation)
   ├─> Receives QueueUpdate via gRPC (<10ms latency) ✅
   ├─> Waits for active phase slot ⏳
   ├─> Checks eligibility
   ├─> Fetches queue data from indexer REST API (50-200ms) 🟡
   ├─> Builds TreeState from indexer data (10-50ms) 🟡
   ├─> Prepares circuit inputs (50-200ms) 🟡
   ├─> Sends ZKP proof requests to prover
   ├─> Polls prover for completion (10-60s) 🔴 BOTTLENECK
   └─> Submits transaction

5. PROVER SERVICE (External)
   ├─> Receives proof request
   ├─> Generates ZKP proof (10-60s) 🔴 DOMINANT LATENCY
   └─> Returns proof

6. SOLANA
   └─> Executes forester tx → Updates tree → Cycle repeats
```

### Components Analysis

#### Photon Indexer (`../photon/src/`)

**Key Files**:
- `events.rs`: Event bus for real-time notifications
- `grpc/event_subscriber.rs`: Converts ingestion events to gRPC streams
- `grpc/queue_monitor.rs`: Polling fallback (monitors DB state)
- `ingester/mod.rs`: Block ingestion and transaction parsing

**Event Flow**:
```rust
// events.rs (L9-41)
pub enum IngestionEvent {
    OutputQueueInsert { tree, queue, count, slot },
    NullifierQueueInsert { tree, queue, count, slot },
    AddressQueueInsert { tree, queue, count, slot },
}

// Published IMMEDIATELY during transaction parsing
// ../photon/src/ingester/parser/*.rs
```

**gRPC Service**:
```rust
// grpc/event_subscriber.rs (L26-101)
// Subscribes to IngestionEvent → broadcasts QueueUpdate
// PUSH model: forester gets notified instantly (<10ms)

// grpc/queue_monitor.rs (L34-98)
// Polls DB every poll_interval_ms
// Fallback when event bus unavailable
```

**Latency Characteristics**:
- Event notification: <10ms from tx confirmation ✅
- DB query (REST API): 50-200ms (depends on data size) 🟡
- Data freshness: Real-time (events) or poll_interval (fallback)

#### Forester (`forester/src/`)

**Current State**:
- ✅ Receives real-time gRPC queue updates and routes them per tree.
- ✅ Persistent staging-tree caches keep roots chained across iterations.
- ✅ Speculative pipeline can prepare proofs early using the same helpers as the main loop.
- ⚠️ Triggering, invalidation, and telemetry for speculation are still immature, so we often waste prover capacity or fall back to the slow path.
- ⚠️ Controller lifecycle is tied to epoch churn; we want long-lived coordinators + tidier cache/state abstractions.

**Key Files**:
- `epoch_manager.rs`: Main orchestrator
  - L1162-1230: Creates persistent StateTreeCoordinator (new!)
  - L1371-1550: Process light slots with gRPC events
- `processor/v2/coordinator/state_tree_coordinator.rs`: Tree processing
  - L788-876: `prepare_batches_streaming()` - builds tree and circuit inputs
  - L96-235: Main process loop
- `grpc/router.rs`: Queue event routing

---

## Identified Bottlenecks

### 🔴 BOTTLENECK 1: ZKP Proving Time (10-60s)
- **Impact**: CRITICAL - Dominant latency source
- **Location**: Prover service (external)
- **Current**: Sequential wait for each proof
- **Cannot optimize**: Proving time is fixed by circuit complexity
- **CAN optimize**: When we START proving (before our slot!)

### 🔴 BOTTLENECK 2: Sequential Processing Model
- **Impact**: CRITICAL - Delays proof generation
- **Current Flow**:
  ```
  Wait for slot → Fetch data → Build tree → Generate proofs → Submit
                   ↑_______________ ALL AFTER SLOT BEGINS _______________↑
  ```
- **Problem**: We can't start until our slot, wasting 10-60s
- **Solution**: Speculative execution

### 🟡 BOTTLENECK 3: Tree State Rebuild / Cache Instrumentation
- **Impact**: MEDIUM (200-500 ms per cache miss)
- **Location**: `fetch_queues_with_accounts()` → `StagingTree`
- **Current**: Cache exists (global + per-coordinator) but lacks visibility, metrics, and proactive invalidation hooks.
- **Problem**: We cannot tell when cache reuse happens or why it was invalidated, so we still rebuild more often than necessary and bugs slip in unnoticed.
- **Solution**: Add instrumentation + telemetry, feed queue updates into the cache, and wire it into the speculative manager so we only rebuild when roots diverge or new batches arrive.

### 🟡 BOTTLENECK 4: Indexer REST API Latency
- **Impact**: MEDIUM (50-200ms)
- **Location**: `fetch_indexed_queues()` HTTP calls
- **Current**: Synchronous HTTP round-trips
- **Solution**: Tree account subscription for freshness checks

### 🟡 BOTTLENECK 5: Circuit Input Preparation
- **Impact**: LOW-MEDIUM (50-200ms)
- **Location**: `batch_preparation.rs`
- **Current**: Single-threaded tree operations
- **Solution**: Parallelization (future optimization)

### ✅ STRENGTH: Real-time Notifications
- **Status**: Already implemented
- **Latency**: <10ms from tx to notification
- **Implementation**: gRPC push model via `QueueEventRouter`

---

## Zero-Latency Architecture Design

### Core Concept: Speculative Proof Generation

**Key Insight**: We receive queue notifications instantly via gRPC. We can predict when our slot will be eligible and start generating proofs BEFORE the slot arrives.

```
Traditional Flow:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Queue Update → Wait for Slot → Fetch → Build → ZKP (60s) → Submit
                                        ↑________________________↑
                                        ALL HAPPENS AFTER SLOT BEGINS

Optimized Flow:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Queue Update → Fetch → Build → ZKP (60s) → Wait for Slot → Submit INSTANTLY
               ↑_____________________________↑
               HAPPENS BEFORE SLOT (SPECULATIVE)
```

### State Machine Design

```rust
enum ForesterState {
    /// No work available
    Idle,

    /// Queue has work, we're warming up cache
    WarmingUp {
        tree_cache: PersistentTreeCache,
        pending_work: QueueUpdateMessage,
    },

    /// Actively generating proofs (speculatively)
    SpeculativeProving {
        tree_cache: PersistentTreeCache,
        proof_futures: Vec<ProofHandle>,
        target_slot: u64,
        work_snapshot: WorkSnapshot,
    },

    /// Proofs ready, waiting for eligible slot
    ReadyToSubmit {
        proofs: Vec<GeneratedProof>,
        circuit_inputs: Vec<CircuitInput>,
        eligible_slot: u64,
    },

    /// Transaction submitted, waiting for confirmation
    Submitting {
        tx_signature: Signature,
        submitted_slot: u64,
    },
}
```

### Persistent Tree Cache Design

```rust
/// Maintains tree state across processing iterations
/// Invalidates only when necessary (root change or new queue items)
struct PersistentTreeCache {
    /// Cached staging tree with accumulated state
    staging_tree: StagingTree,

    /// Base root this cache was built from
    base_root: [u8; 32],

    /// Last Solana slot we synced with
    last_synced_slot: u64,

    /// Queue updates received since last cache build
    pending_queue_updates: Vec<QueueUpdate>,

    /// Last on-chain tree state snapshot
    last_known_tree_state: OnchainTreeState,
}

impl PersistentTreeCache {
    /// Apply queue update notification (from gRPC)
    fn on_queue_update(&mut self, update: QueueUpdate) {
        // Track that new work exists
        // Don't rebuild yet - wait until we need to process
        self.pending_queue_updates.push(update);
    }

    /// Check if cache is valid for reuse
    fn is_valid(&self, current_root: [u8; 32]) -> bool {
        // Valid if:
        // 1. Root unchanged (no other forester processed)
        // 2. No pending updates (no new leaves to process)
        self.base_root == current_root
            && self.pending_queue_updates.is_empty()
    }

    /// Invalidate cache and rebuild from fresh indexer data
    fn rebuild(&mut self, fresh_tree_state: TreeState) {
        self.staging_tree = fresh_tree_state.create_staging();
        self.base_root = fresh_tree_state.get_cached_root();
        self.pending_queue_updates.clear();
    }

    /// Check if on-chain state changed (another forester updated)
    fn sync_with_chain(&mut self, current_root: [u8; 32]) -> bool {
        let changed = self.base_root != current_root;
        if changed {
            // Mark cache stale - will rebuild on next use
            self.base_root = current_root;
        }
        changed
    }
}
```

### Speculative Engine (Existing Implementation)

`SpeculativeEngine` already exists inside `forester/src/processor/v2/coordinator/state_tree_coordinator.rs`. It tracks queue updates per tree, guards an inflight preparation, and stores a finished `SpeculativeJob` (pattern, proofs, staging snapshot, metrics). The normal coordinator loop calls:

- `prepare_speculative_job()` whenever new queue data and idle prover capacity coincide. This clones the staging tree, invokes `prepare_iteration_inputs()` (same helper used by the regular processing loop), computes append/nullify proofs, and persists them to the `SpeculativeEngine`.
- `try_execute_speculative_job()` every slot iteration. If a job exists, it validates the stored root against the live on-chain account, submits the proofs immediately, and falls back to the traditional `process_batches()` path only when the job was invalidated or incomplete.

**Gaps Identified**

1. **Triggering discipline** – Every queue update currently schedules speculation, even when the next eligible slot is minutes away. We need heuristics tied to epoch schedule and queue depth to prevent prover spam and ensure work is ready roughly 40 s before the slot, per product requirement.
2. **Hard invalidation hooks** – Cache invalidations already happen when the root changes, but the speculative job is not dropped immediately, so we risk replaying stale proofs. Root updates, queue deltas, or prover errors should call `SpeculativeEngine::reset()` with explicit reasons.
3. **Observability** – There is almost no telemetry showing job lifecycles. We need counters/latency histograms for “job prepared”, “job executed”, “job invalidated (reason=queue/root/prover)”, and “job wasted”, plus structured logs keyed by tree.
4. **Concurrency & cancellation** – The engine tracks a single inflight job; if the prover takes >40 s, speculation blocks for the entire tree. We should either add cancellation hooks (tell the prover to drop requests when invalidated) or support multiple staged jobs keyed by eligible slot so we always have the freshest data ready.
5. **Indexer waits** – The code now only calls `wait_for_indexer` immediately before Photon RPC/REST interactions, but we still need a clear audit to ensure no speculative path bypasses this guard, otherwise we may build proofs on stale queue snapshots.

---

## Implementation Roadmap

### Sprint 1: Smart Cache with Proper Invalidation (✅ COMPLETE)
**Duration**: 2-4 hours  
**Status**: Shipped (Nov 2024)  
**Result**: Persistent staging caches are back, roots stay chained, and constraint #13029 is resolved. Remaining work is observability + policy tuning (captured in the action plan).

---

enum SpeculativeState {
    Idle,
    Proving {
        proof_handles: Vec<ProofHandle>,
        started_at_slot: u64,
    },
    Ready {
        proofs: Vec<Proof>,
        valid_until_slot: u64,
    },
}
### Sprint 2: Speculative Proof Generation
**Duration**: 1-2 days (initial plumbing) + ongoing tuning  
**Status**: **In progress** – `SpeculativeEngine` exists and can submit pre-built jobs, but heuristics/invalidation/telemetry pending.  
**Priority**: CRITICAL (biggest latency win)

**Objectives (updated)**:
1. Finalize triggering policy (slot prediction + backlog heuristics) so speculation starts only when we are within ~40 s of the next eligible slot.
2. Wire hard invalidation hooks: queue deltas, root changes, explicit prover errors must reset the engine and cancel inflight proof requests.
3. Add structured metrics/logs for speculative lifecycle plus feature flags to compare fallback vs. speculative submissions.
4. (Optional for this sprint) Support multi-job buffering or explicit prover cancellation to keep proofs fresh even if earlier jobs are invalidated late.

**Implementation Notes**:
- All logic now lives in `forester/src/processor/v2/coordinator/state_tree_coordinator.rs` and shares helpers with the normal loop (e.g., `prepare_iteration_inputs`).
- Slot timing data is available via the epoch manager; we should expose a helper that returns “seconds until eligible slot” and feed that into the coordinator.
- Invalidation should be orchestrated by `StateTreeCoordinator::handle_queue_update` & `StateTreeCoordinator::invalidate_cache`, so caches, staging trees, and speculation stay coherent.

**Validation**:
- Emit telemetry for preparation latency, success/invalidation counts, and submission gap.  
- In the devnet/localnet e2e test, verify ≥60 % of batches submit <100 ms after eligibility; log invalidation reasons for the remainder.

---

### Sprint 3: Direct Tree Account Subscription
**Duration**: 4-8 hours
**Status**: Independent, can run parallel
**Priority**: MEDIUM (reduces false invalidations)

**Objectives**:
1. Subscribe to merkle tree account changes via Solana RPC
2. Parse root directly from account data
3. Invalidate speculative work immediately when root changes
4. Bypass indexer for freshness checks

**Implementation**:

```rust
// forester/src/account_subscriber.rs (new)

pub struct TreeAccountSubscriber {
    rpc: Arc<dyn Rpc>,
    tree_pubkey: Pubkey,
    update_tx: mpsc::Sender<TreeUpdate>,
}

impl TreeAccountSubscriber {
    pub async fn start(self) {
        // Subscribe to account changes
        let mut account_stream = self.rpc
            .account_subscribe(self.tree_pubkey)
            .await
            .expect("Failed to subscribe");

        while let Some(account) = account_stream.next().await {
            // Parse root from account data
            let root = Self::parse_root(&account.data);

            // Notify speculative prover
            self.update_tx.send(TreeUpdate { root }).await;
        }
    }

    fn parse_root(data: &[u8]) -> [u8; 32] {
        // Parse BatchedMerkleTreeAccount to extract current root
        // Faster than querying indexer
        todo!()
    }
}
```

**Integration**:
- `StateTreeCoordinator` / `SpeculativeEngine`: Subscribe to tree updates
- Invalidate speculative work immediately when root changes

**Expected Gains**:
- 100-500ms faster invalidation detection
- Reduce unnecessary speculative work

---

### Sprint 4: Circuit Input Optimization
**Duration**: 1-2 days
**Status**: Future optimization
**Priority**: LOW (smaller impact)

**Objectives**:
1. Profile `batch_preparation.rs` to find hotspots
2. Parallelize proof generation per leaf
3. Use SIMD for Poseidon hash operations
4. Pre-compute common Merkle paths

**Implementation**:
- Requires profiling to identify bottlenecks
- Likely candidates:
  - `prepare_append_batch()`
  - `prepare_nullify_batch()`
  - Tree proof generation

**Expected Gains**:
- 50-200ms reduction in circuit input preparation

---

## Success Metrics

### Before Optimization
- **Time to submit after slot eligible**: 10-60 seconds
- **Tree rebuild frequency**: Every iteration
- **Cache hit rate**: 0% (disabled)

### After Sprint 1 (Smart Cache)
- **Cache hit rate**: 50-80% (when waiting for new work)
- **Time saved per cache hit**: 200-500ms

### After Sprint 2 (Speculative Proving)
- **Time to submit after slot eligible**: <100ms
- **Speculative success rate**: 60-90% (proofs valid when slot arrives)
- **Total latency reduction**: **10-60s → <1s**

### After Sprint 3 (Tree Subscription)
- **Invalidation detection latency**: 50-100ms (from 200-500ms)
- **Speculative success rate**: 70-95% (better invalidation)

---

## Risk Analysis

### Risk 1: Wasted Computation
**Scenario**: Speculative proofs generated but invalidated before use
**Mitigation**:
- Track invalidation rate
- Only start speculative work if high confidence
- Implement proof cancellation to free prover resources

### Risk 2: Race Conditions
**Scenario**: Multiple foresters generate same proofs speculatively
**Impact**: Wasted resources, but functionally correct (only one tx succeeds)
**Mitigation**:
- Slot eligibility already provides ordering
- Monitor duplicate work in metrics

### Risk 3: Cache Staleness Bugs
**Scenario**: Cache not invalidated when needed → wrong proofs
**Mitigation**:
- Comprehensive validation before tx submission
- Always verify on-chain root matches expected root
- Existing root validation in `validate_root()`

### Risk 4: Prover Resource Exhaustion
**Scenario**: Too many speculative requests overwhelm prover
**Mitigation**:
- Rate limiting on speculative requests
- Priority queue (actual slot > speculative)
- Implement proof request cancellation

---

## Rollout Strategy

### Phase 1: Foundation (Week 1)
- ✅ Implement Sprint 1 (Smart Cache)
- ✅ Run e2e tests
- ✅ Deploy to testnet
- ✅ Monitor cache hit rates

### Phase 2: Speculative Core (Week 2)
- Implement Sprint 2 (Speculative Proving)
- Start with conservative prediction (low speculation)
- Monitor invalidation rates
- Gradually increase speculation aggressiveness

### Phase 3: Optimization (Week 3)
- Implement Sprint 3 (Tree Subscription)
- Tune speculation parameters
- Performance testing under load

### Phase 4: Production (Week 4)
- Deploy to mainnet with feature flag
- A/B test: 50% traditional, 50% speculative
- Monitor metrics and gradually roll out

---

## Action Plan (Updated)

| # | Workstream | Description | Success Criteria | Target |
|---|------------|-------------|------------------|--------|
| 1 | Coordinator Observability | Emit metrics/logs for cache hits, speculation lifecycle (prepare/execute/invalidate + reason), queue backlog, and controller latency. Wire counters into `forester_staging_cache_events_total` & new Prometheus gauges. | Grafana shows speculation success %, invalidation reasons, and cache hit rate; e2e log noise reduced; failures include a trace-id. | 2 days |
| 2 | Speculation Triggering | Implement slot/backlog heuristics so `prepare_speculative_job()` only fires when within ~40 s of eligibility or when queue depth exceeds N. Surface knobs via config/env. | Localnet run shows ≤1 speculative request per slot per tree; at least 60% of eligible slots have ready proofs. | 3 days |
| 3 | Hard Invalidation & Cancellation | Drop speculative jobs instantly when roots change, queue deltas arrive, or prover errors occur. Send cancellation to prover (once supported) to free capacity. | No stale job can execute after a root change; telemetry shows invalidations with explicit reason; prover backlog stable under churn. | 3 days (after #2) |
| 4 | Architecture Cleanup | Keep one `StateTreeCoordinator` per tree for the lifetime of the epoch manager, merge redundant cache/staging structures if possible, and remove noisy logs. Document lifecycle in `epoch_manager.rs`. | Controller registry no longer refreshes each epoch; logs trimmed; doc outlines how caches/state trees interact. | 4 days |
| 5 | Indexer & Root Watchers | Enforce `wait_for_indexer` placement by linting plus add RPC account subscriptions per tree to trigger invalidation faster than REST polls. | Account-subscriber invalidations visible in logs; Photon calls guarded by waits; speculation success improves ≥10 %. | 3 days |
| 6 | Prover/Server Alignment | After forester-side speculation stabilizes, experiment with prover-side completed-queue reuse so repeated inputs skip regeneration. Design handshake protocol. | Prototype demonstrates prover cache hit serving an identical append job; decision doc comparing forester speculation vs prover caching. | 1 week (post #3) |

**Owner**: Forester core team (coordination between Rust + infra).  
**Tracking**: Create GitHub issues per row, link to this plan, and update status weekly.

---

## Conclusion

The forester's dominant latency is ZKP proving time (10-60s). We cannot make proving faster, but we can **start proving earlier** through speculative execution.

**Key enabler**: We already have real-time gRPC notifications (<10ms latency) when queue work arrives.

**Key innovation**: Start generating proofs BEFORE our eligible slot, so they're ready to submit instantly when our turn arrives.

**Expected outcome**: Reduce forester submission latency from 10-60s to <1s.

**Implementation complexity**: Medium (2-3 weeks of focused development)

**Risk**: Low (can gracefully fall back to traditional flow if speculation fails)

---

## Appendix: Code References

### Photon Indexer
- Event system: `../photon/src/events.rs`
- gRPC subscriber: `../photon/src/grpc/event_subscriber.rs`
- Queue monitor: `../photon/src/grpc/queue_monitor.rs`
- Ingestion: `../photon/src/ingester/mod.rs`

### Forester
- Epoch manager: `forester/src/epoch_manager.rs`
- State coordinator: `forester/src/processor/v2/coordinator/state_tree_coordinator.rs`
- Batch preparation: `forester/src/processor/v2/coordinator/batch_preparation.rs`
- Tree state: `forester/src/processor/v2/coordinator/tree_state.rs`
- gRPC router: `forester/src/grpc/router.rs`

### On-chain Programs
- Batched merkle tree: `programs/batched-merkle-tree/`
- Account compression: `programs/account-compression/`

---

**Next Step**: Tackle Action Plan item #2 (speculation triggering heuristics) now that cache/speculation instrumentation is in place.
