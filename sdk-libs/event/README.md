<!-- cargo-rdme start -->

# light-event

Event types and parsing for Light Protocol transactions.

| Type | Description |
|------|-------------|
| [`PublicTransactionEvent`](event::PublicTransactionEvent) | Transaction event with input/output compressed account hashes |
| [`BatchPublicTransactionEvent`](event::BatchPublicTransactionEvent) | Batched event with accounts, addresses, and sequence numbers |
| [`event_from_light_transaction`](parse::event_from_light_transaction) | Parse transaction instructions into a batch event |

<!-- cargo-rdme end -->
