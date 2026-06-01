<div align="center">
  <img src="logo.svg" width="420" alt="Oxivend">
</div>

<p align="center">
  A lightweight, headless BaaS for selling, distributing, and licensing digital products.
  <br>
  Built with Rust &middot; Backed by PostgreSQL &middot; Apache 2.0
</p>

<p align="center">
  <a href="#features">Features</a> &middot;
  <a href="#quick-start">Quick Start</a> &middot;
  <a href="#api">API</a> &middot;
  <a href="#architecture">Architecture</a> &middot;
  <a href="#configuration">Configuration</a> &middot;
  <a href="#development">Development</a> &middot;
  <a href="#license">License</a>
</p>

---

**Oxivend** provides independent creators with a turnkey platform to securely sell, distribute, and license digital products — from software licenses to downloadable assets. The initial release focuses on the software licensing engine, with asset delivery following as a separate module.

- **Single binary** — minimal dependencies, low resource usage
- **Self-hosted** — full control of your infrastructure and data
- **Ed25519 cryptography** — signed license tokens for online and offline verification
- **Docker-native** — ready-to-run images on GitHub Container Registry

---

## Features

### Software Licensing Engine

| Capability | Description |
|---|---|
| **Product & Key Management** | Register products, generate Ed25519 key pairs per product |
| **Payment Integration** | Stripe webhook handling with signature verification & idempotency |
| **Cryptographic Licenses** | License payload signed with Ed25519; delivered as signed tokens |
| **Online Activation** | Device fingerprinting with configurable activation limits |
| **License Verification** | Short-lived access JWTs; offline client-side verification |
| **Revocation & Blacklist** | Revoke licenses; periodic blacklist sync for offline clients |
| **Flexible Models** | Perpetual, subscription, trial, feature-based, and floating licenses |

### Platform

- **RESTful API** with JSON request/response
- **Structured logging** via `tracing` and Prometheus metrics
- **Role-based access** with JWT + refresh token session management
- **Rate-limited** activation endpoints
- **Structured error responses** with consistent format

---

## Quick Start

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/) and [Docker Compose](https://docs.docker.com/compose/install/)

### Run with Docker Compose

```bash
# Clone the repository
git clone https://github.com/thomalghani/oxivend.git
cd oxivend

# Start PostgreSQL and Oxivend
docker compose -f docker/docker-compose.yml up -d

# Check the health endpoint
curl http://localhost:3000/health
```

The API is now available at `http://localhost:3000`.

### Run from Source

```bash
# Start a PostgreSQL instance
docker run -d --name oxivend-pg \
  -e POSTGRES_USER=oxivend \
  -e POSTGRES_PASSWORD=oxivend_dev \
  -e POSTGRES_DB=oxivend \
  -p 5432:5432 \
  postgres:18-alpine

# Copy and edit configuration
cp .env.example .env

# Run the binary
cargo run --release
```

---

## API

### Public Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check |
| `POST` | `/login` | Admin login |
| `POST` | `/auth/refresh` | Refresh access token |

### Authenticated Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/products` | Create product |
| `GET` | `/products` | List products |
| `GET` | `/products/{id}` | Get product |
| `PUT` | `/products/{id}` | Update product |
| `DELETE` | `/products/{id}` | Soft-delete product |
| `POST` | `/products/{id}/keys` | Generate Ed25519 key pair |
| `POST` | `/licenses` | Issue license |
| `POST` | `/licenses/verify` | Activate or verify license |
| `POST` | `/licenses/{id}/revoke` | Revoke license |

### Webhooks

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/webhooks/payment/stripe` | Stripe payment notification |

---

## Architecture

```
                    ┌──────────────┐
                    │  Creator CLI  │
                    │  / Dashboard  │
                    └──────┬───────┘
                           │
              ┌────────────┴────────────┐
              │     REST API Layer      │
              │   (axum, JWT auth)      │
              └────────────┬────────────┘
                           │
              ┌────────────┴────────────┐
              │   Licensing Engine      │
              │  (Ed25519, activation)  │
              └────────────┬────────────┘
                           │
              ┌────────────┴────────────┐
              │      PostgreSQL         │
              └─────────────────────────┘

    ┌──────────────┐       ┌───────────────────┐
    │  End-User    │       │  Payment Gateway  │
    │  App (SDK)   │       │  Stripe / etc.    │
    └──────┬───────┘       └─────────┬─────────┘
           │ activate / verify       │ webhook
           └──────────┬──────────────┘
                      │
              ┌───────┴───────┐
              │  Oxivend API  │
              └───────────────┘
```

### Project Structure

```
oxivend/
├── Cargo.toml            # single package (not a workspace)
├── src/
│   ├── main.rs           # binary entrypoint
│   ├── lib.rs            # shared library root
│   ├── crypto.rs         # Ed25519 license operations
│   ├── auth.rs           # password hashing, sessions, seed
│   ├── db.rs             # pool, migrations
│   ├── errors.rs         # all error types
│   ├── config.rs         # environment configuration
│   ├── state.rs          # application state
│   └── routes/           # HTTP handlers
│       ├── mod.rs
│       ├── health.rs
│       ├── auth.rs
│       └── products.rs
├── migrations/           # SQL migrations
├── tests/                # unit & integration tests
├── docker/
│   ├── Dockerfile        # multi-stage build
│   └── docker-compose.yml
└── logo.svg              # brand wordmark
```

---

## Configuration

All configuration is via environment variables (see `.env.example`):

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | — |
| `DATABASE_POOL_SIZE` | Connection pool size | `10` |
| `OXIVEND_ADMIN_EMAIL` | Initial admin email | — |
| `OXIVEND_ADMIN_PASSWORD` | Initial admin password | — |
| `JWT_SECRET` | Secret for signing JWTs | — |
| `PRIVATE_KEY_ENCRYPTION_KEY` | AES-256-GCM key for private keys | — |
| `ACCESS_TOKEN_TTL_SECONDS` | JWT access token TTL | `900` |
| `REFRESH_TOKEN_TTL_DAYS` | Refresh token TTL | `7` |
| `REFRESH_TOKEN_BYTES` | Refresh token entropy | `32` |

On first boot with no existing users, an admin account is seeded automatically from the `OXIVEND_ADMIN_EMAIL` and `OXIVEND_ADMIN_PASSWORD` variables.

---

## Development

### Prerequisites

- Rust 2024 edition (MSRV: 1.85+)
- PostgreSQL 16+

### Setup

```bash
# Install dependencies
cargo fetch

# Run database migrations (applies automatically at startup)
cargo run

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt --check
```

### Database Schema

Six core tables:

| Table | Purpose |
|---|---|
| `users` | Admin accounts |
| `refresh_tokens` | JWT refresh token management |
| `products` | Digital products with Ed25519 key pairs |
| `licenses` | Issued licenses tied to products & customers |
| `activations` | Device activation records with fingerprints |
| `purchases` | Payment gateway purchase records (idempotency key) |

See [`docs/internal/schema.md`](docs/internal/schema.md) for full details.

---

## Logos

| Variant | Preview |
|---|---|
| Orange | [`shield-orange.svg`](shield-orange.svg) |
| Black | [`shield-black.svg`](shield-black.svg) |
| Wordmark | [`logo.svg`](logo.svg) |

---

## License

Apache 2.0 &mdash; see [LICENSE](LICENSE).

---

<p align="center">
  <sub>Built with ❤️ and Rust</sub>
</p>
