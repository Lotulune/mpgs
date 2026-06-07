# Use SQLx for Postgres Access

The public discovery service will use SQLx for Postgres access rather than an ORM. The service needs explicit SQL for paginated public queries, worker job claiming, locks, migrations, and operations reporting, so keeping SQL visible is preferable to hiding the database model behind generated entities.

**Consequences**

The server must maintain explicit SQL migrations and query mapping code. If compile-time SQL checking is enabled, local development and CI need a database or SQLx offline metadata.
