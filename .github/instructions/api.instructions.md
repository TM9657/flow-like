---
applyTo: "packages/api/**/*.rs"
---
# API instructions

## Security and Permissions
First of all it is very important that you always verify access of the user to the resources. Both with the initial ensure_permission!(user, &app_id, &state, RolePermissions::<Permission>); for project as well as in the SQL queries. Check comparable endpoints before implementing new ones.

## OpenAPI Specification
Always describe the endpoint using the utoipa annotations. This is important for documentation and for generating client SDKs. If you are adding a new endpoint, make sure to include the appropriate annotations for request and response types.

Also give each of the endpoints a short description. Target audience is end users.

## Performance
Keep Queries as optimized as possible, use caching where appropriate, and avoid unnecessary database calls. Always consider the performance implications of your implementation, especially for endpoints that may be called frequently.