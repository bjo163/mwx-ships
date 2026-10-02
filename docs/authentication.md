# Authentication in Moonships

Moonships uses Loco.rs standard JSON Web Token (JWT) authentication for API security.

## Authentication Mechanisms

1. **Email / Password**:
   - `POST /api/auth/register`: Register operator account.
   - `POST /api/auth/login`: Authenticate and receive a signed JWT token.
   - Passwords hashed using Argon2id with random salt.
2. **Bearer Token Authentication**:
   - Every protected API request must include the header:
     ```http
     Authorization: Bearer <JWT_TOKEN>
     ```
3. **Current User Profile**:
   - `GET /api/auth/current`: Returns authenticated operator profile and PID.

## Token Expiration & Storage
- JWT expiration is configurable via `config/*.yaml` under `auth.jwt.expiration`.
- The React dashboard stores the token in `localStorage` under `moonships_token` and attaches it to all API calls via `src/api/client.ts`.
