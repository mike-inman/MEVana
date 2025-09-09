# MEVana API Service

A comprehensive REST API service for the MEVana arbitrage trading system, built with Axum and featuring complete OpenAPI/Swagger documentation.

## Features

- 🚀 **High Performance**: Built with Axum for maximum performance and concurrency
- 📊 **Complete API Coverage**: Access all MEVana data through REST endpoints
- 📖 **OpenAPI/Swagger**: Full API documentation with interactive UI
- 🔒 **Security**: JWT authentication and authorization
- 🛡️ **Resilience**: Built-in retry logic, circuit breakers, and graceful degradation
- 📈 **Monitoring**: Comprehensive metrics and health checks
- 🔍 **Observability**: Request tracing and structured logging

## Quick Start

### Environment Variables

```bash
DATABASE_URL=postgresql://user:password@localhost:5432/mevana
PORT=8080
JWT_SECRET=your-secret-key-here
RUST_LOG=info
```

### Running the Server

```bash
# Development
cargo run --bin api-server

# Production build
cargo build --release --bin api-server
./target/release/api-server
```

### Docker

```bash
# Build image
docker build -t mevana-api .

# Run container
docker run -p 8080:8080 \
  -e DATABASE_URL=postgresql://host.docker.internal:5432/mevana \
  mevana-api
```

## API Documentation

Once the server is running, visit:

- **Swagger UI**: http://localhost:8080/swagger-ui
- **OpenAPI Spec**: http://localhost:8080/api-docs/openapi.json
- **Health Check**: http://localhost:8080/health

## Endpoints Overview

### Wallets (`/api/v1/wallets`)
- `GET /wallets` - List all wallets with pagination
- `GET /wallets/{id}` - Get wallet by ID

### Arbitrage (`/api/v1/arbitrage`)
- `GET /arbitrage/opportunities` - List arbitrage opportunities
- `GET /arbitrage/opportunities/{id}` - Get opportunity details
- `GET /arbitrage/executions` - List execution results

### Metrics (`/api/v1/metrics`)
- `GET /metrics/performance` - Performance metrics over time
- `GET /metrics/health` - System health status
- `GET /metrics/trading-performance` - Trading performance summary

### Tokens (`/api/v1/tokens`)
- `GET /tokens` - Token registry
- `GET /tokens/{mint}` - Get token by mint address

### Statistics (`/api/v1/statistics`)
- `GET /statistics/venues` - Venue performance statistics
- `GET /statistics/token-pairs` - Token pair statistics
- `GET /statistics/wallet-performance` - Wallet performance metrics

## Query Parameters

Most endpoints support pagination and filtering:

```bash
# Pagination
GET /api/v1/wallets?limit=50&offset=0

# Filtering
GET /api/v1/arbitrage/opportunities?venue=orca&min_profit_bps=25

# Time range
GET /api/v1/metrics/performance?from_time=2024-01-01T00:00:00Z&to_time=2024-01-02T00:00:00Z
```

## Response Format

All API responses follow a consistent format:

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2024-01-01T00:00:00Z"
}
```

### Paginated Responses

```json
{
  "success": true,
  "data": {
    "data": [...],
    "pagination": {
      "offset": 0,
      "limit": 50,
      "total": 1000,
      "has_more": true
    }
  },
  "error": null,
  "timestamp": "2024-01-01T00:00:00Z"
}
```

## Authentication

The API supports JWT-based authentication:

```bash
# Login (if user management is enabled)
POST /api/v1/auth/login
{
  "username": "user@example.com",
  "password": "password"
}

# Use token in requests
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/wallets
```

## Error Handling

Errors are returned with appropriate HTTP status codes and detailed messages:

```json
{
  "success": false,
  "data": null,
  "error": "Wallet not found",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

Common status codes:
- `200` - Success
- `400` - Bad Request
- `401` - Unauthorized
- `404` - Not Found
- `500` - Internal Server Error

## Rate Limiting

The API includes built-in rate limiting to prevent abuse:

- Default: 100 requests per minute per IP
- Authenticated users: 1000 requests per minute
- Headers include rate limit status

## Monitoring and Observability

### Health Checks

```bash
GET /health
# Returns: "OK" (200) or error (5xx)
```

### Metrics

Prometheus metrics are exposed at `/metrics` (if enabled):

- Request counts and latencies
- Database connection pool status
- Circuit breaker states
- Custom business metrics

### Request Tracing

Each request gets a unique ID for tracing:

```bash
curl -v http://localhost:8080/api/v1/wallets
# Response includes: X-Request-ID: 123e4567-e89b-12d3-a456-426614174000
```

## Configuration

### Database

The API uses PostgreSQL with connection pooling and resilience features:

```bash
DATABASE_URL=postgresql://user:password@localhost:5432/mevana
DB_MAX_CONNECTIONS=20
DB_CONNECT_TIMEOUT=30
```

### Security

```bash
JWT_SECRET=your-very-secure-secret-key
CORS_ORIGINS=http://localhost:3000,https://app.mevana.com
```

### Performance

```bash
COMPRESSION_ENABLED=true
CACHE_TTL=300
MAX_REQUEST_SIZE=10MB
```

## Development

### Testing

```bash
# Run all tests
cargo test

# Integration tests
cargo test --test integration

# Load testing with k6
k6 run load-test.js
```

### Code Generation

OpenAPI client libraries can be generated using the spec:

```bash
# TypeScript/JavaScript
openapi-generator generate -i openapi.json -g typescript-fetch -o ./client

# Python
openapi-generator generate -i openapi.json -g python -o ./python-client

# Go
openapi-generator generate -i openapi.json -g go -o ./go-client
```

## Deployment

### Production Checklist

- [ ] Set strong `JWT_SECRET`
- [ ] Configure proper `DATABASE_URL`
- [ ] Enable TLS/HTTPS
- [ ] Set up monitoring and alerting
- [ ] Configure log aggregation
- [ ] Set resource limits
- [ ] Enable backup and recovery

### Container Deployment

```yaml
# docker-compose.yml
version: '3.8'
services:
  api:
    image: mevana-api:latest
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://postgres:password@db:5432/mevana
      - JWT_SECRET=your-secret
    depends_on:
      - db
      
  db:
    image: postgres:15
    environment:
      POSTGRES_DB: mevana
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

## Troubleshooting

### Common Issues

1. **Database Connection Errors**
   ```
   Error: Failed to connect to database
   Solution: Check DATABASE_URL and ensure PostgreSQL is running
   ```

2. **High Memory Usage**
   ```
   Issue: API consuming excessive memory
   Solution: Adjust DB_MAX_CONNECTIONS and enable compression
   ```

3. **Slow Responses**
   ```
   Issue: API responses are slow
   Solution: Check database indexes and connection pool size
   ```

### Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run --bin api-server
```

Check application metrics:

```bash
curl http://localhost:8080/metrics/health
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Update API documentation
5. Submit a pull request

## License

MIT License - see LICENSE file for details.