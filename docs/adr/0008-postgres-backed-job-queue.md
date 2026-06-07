# Use Postgres-Backed Jobs for Server Tasks

The public discovery service will store scheduled work, manual job runs, failures, and worker progress in Postgres rather than introducing Redis or a separate queue for the first server release. The workload is dominated by Steam, LLM, metadata backfill, and analysis jobs where observability and self-hosting simplicity matter more than high-throughput message delivery.

**Consequences**

Workers should claim jobs through transactional Postgres patterns such as row locks or advisory locks. Redis or a dedicated queue can be revisited if task volume, scheduling latency, or horizontal scaling needs outgrow Postgres-backed jobs.
