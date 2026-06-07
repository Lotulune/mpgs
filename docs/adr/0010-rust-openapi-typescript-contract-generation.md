# Generate TypeScript Contracts from Rust OpenAPI

API contract types will be defined at the Rust service boundary, exposed through an OpenAPI document, and generated into TypeScript for the user client and management UI. This replaces the current pattern of manually keeping frontend TypeScript types and Rust Tauri command models aligned.

**Consequences**

The server crate should treat OpenAPI generation as part of its public contract. Client and admin frontend code should depend on generated API types or clients rather than hand-written copies of response shapes.
