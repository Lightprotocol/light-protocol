export interface ForesterInfo {
  authority: string;
  balance_sol: number | null;
}

export interface BatchInfo {
  batch_index: number;
  batch_state: number;
  num_inserted: number;
  current_index: number;
  pending: number;
  items_in_current_zkp_batch: number;
}

export interface V2QueueInfo {
  next_index: number;
  pending_batch_index: number;
  zkp_batch_size: number;
  batches: BatchInfo[];
  input_pending_batches: number;
  output_pending_batches: number;
  input_items_in_current_zkp_batch: number;
  output_items_in_current_zkp_batch: number;
}

export interface TreeStatus {
  tree_type: string;
  merkle_tree: string;
  queue: string;
  fullness_percentage: number;
  next_index: number;
  capacity: number;
  height: number;
  threshold: number;
  is_rolledover: boolean;
  queue_length: number | null;
  v2_queue_info: V2QueueInfo | null;
  assigned_forester: string | null;
  schedule: (number | null)[];
  owner: string;
}

export interface AggregateQueueStats {
  state_v1_total_pending: number;
  state_v2_input_pending: number;
  state_v2_output_pending: number;
  address_v1_total_pending: number;
  address_v2_input_pending: number;
}

export interface ForesterStatus {
  slot: number;
  current_active_epoch: number;
  current_registration_epoch: number;
  active_epoch_progress: number;
  active_phase_length: number;
  active_epoch_progress_percentage: number;
  hours_until_next_epoch: number;
  slots_until_next_registration: number;
  hours_until_next_registration: number;
  active_epoch_foresters: ForesterInfo[];
  registration_epoch_foresters: ForesterInfo[];
  trees: TreeStatus[];
  current_light_slot: number | null;
  light_slot_length: number;
  slots_until_next_light_slot: number | null;
  total_light_slots: number;
  total_trees: number;
  active_trees: number;
  rolled_over_trees: number;
  total_pending_items: number;
  aggregate_queue_stats: AggregateQueueStats;
}

export interface MetricsResponse {
  transactions_processed_total: Record<string, number>;
  transaction_rate: Record<string, number>;
  last_run_timestamp: number;
  forester_balances: Record<string, number>;
  queue_lengths: Record<string, number>;
}

export interface CompressibleResponse {
  enabled: boolean;
  ctoken_count?: number;
  ata_count?: number;
  pda_count?: number;
  mint_count?: number;
  current_slot?: number;
  total_tracked?: number;
  total_ready?: number;
  total_waiting?: number;
  ctoken?: CompressibleTypeStats;
  ata?: CompressibleTypeStats;
  pda?: CompressibleTypeStats;
  mint?: CompressibleTypeStats;
  pda_programs?: PdaProgramStats[];
  upstreams?: CompressibleUpstreamStatus[];
  note?: string;
  error?: string;
  refresh_interval_secs?: number;
  source?: string;
  cached_at?: number;
}

export interface CompressibleTypeStats {
  tracked: number;
  ready?: number;
  waiting?: number;
  next_ready_slot?: number;
}

export interface PdaProgramStats {
  program_id: string;
  tracked: number;
  ready?: number;
  waiting?: number;
  next_ready_slot?: number;
}


export interface CompressibleUpstreamStatus {
  base_url: string;
  ok: boolean;
  source?: string;
  cached_at?: number;
  error?: string;
}
