# Use Single Admin Token with Session Cookies

The first public discovery service release will protect management access with a single administrator token that is configured outside the database and exchanged for a short-lived HttpOnly session cookie. This avoids building a full account system before the product needs multiple administrators, while keeping the browser-based management UI away from long-lived tokens in local storage.

**Consequences**

Anonymous clients can only use public read routes. Management routes require a valid session cookie, and deployment documentation must explain how to set and rotate the administrator token.
