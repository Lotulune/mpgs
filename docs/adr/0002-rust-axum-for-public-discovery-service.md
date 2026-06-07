# Use Rust and Axum for the Public Discovery Service

The public discovery service will be implemented in Rust with Axum rather than rewritten in FastAPI or Node. The existing MPGS backend logic is already Rust code inside the Tauri application, including Steam integration, LLM calls, scoring, discovery jobs, and queues, so Axum lets the project turn that code into HTTP service boundaries without rewriting the core domain behavior in another language.

**Consequences**

The service should extract reusable logic from the current Tauri backend into shared Rust crates before exposing HTTP routes. FastAPI remains a reasonable framework in general, but choosing it here would trade developer ergonomics for a larger migration and duplicated business logic.
