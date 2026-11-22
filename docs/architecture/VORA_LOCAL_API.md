# VORA MIND Local API

## Namespace

All application endpoints use `/api/v1/`.

## Endpoints

- `GET /api/v1/health`
- `GET /api/v1/info`
- `GET /api/v1/capabilities`
- `GET /api/v1/capabilities/{id}`
- `POST /api/v1/capabilities/{id}/execute`
- `GET /api/v1/executions`
- `GET /api/v1/executions/{id}`
- `GET /api/v1/projects`
- `POST /api/v1/projects`
- `GET /api/v1/projects/{id}`
- `PATCH /api/v1/projects/{id}`
- `GET /api/v1/workflows`
- `POST /api/v1/workflows`
- `GET /api/v1/workflows/{id}`
- `POST /api/v1/workflows/{id}/start`
- `POST /api/v1/workflows/{id}/cancel`
- `POST /api/v1/workflows/{id}/resume`
- `POST /api/v1/workflows/{id}/approve_step`
- `POST /api/v1/workflows/{id}/history`

## Security

The server binds to `127.0.0.1` with an ephemeral local port. Protected
endpoints require `Authorization: Bearer <token>`. The health endpoint is
unauthenticated but contains no secret or application data.

The token is generated at API server construction and is never logged or
persisted. UI status deliberately does not expose it. External clients receive
the token through the controlled UI/API enablement flow.
