# Webhook Forwarder

A simple webhook forwarding service built with Rust that allows you to manage and forward webhook events to multiple endpoints.

## Features
- Register multiple webhook endpoints
- Enable/disable endpoints individually
- Forward webhook events to active endpoints
- Simple web interface for management
- Docker support

## Quick Start

### Setup
```bash
# Clone the repository
git clone <repository-url>
cd webhook-dispatcher

# Build and run
cargo run

# Access the UI at http://localhost:8080
# Access the webhook endpoint at http://localhost:8080/webhook
```

## Usage

1. Add webhook endpoints through the UI:
   - Enter endpoint URL
   - Give it a name
   - Toggle active/inactive status

2. Send webhook events to:
```bash
curl -X POST http://localhost:3000/webhook \
  -H "Content-Type: application/json" \
  -d '{"event": "test", "data": "your payload"}'
```

The service will forward the webhook payload to all active endpoints.

## Development

### Prerequisites
- Rust 1.70 or higher
- Docker (optional)

### Local Development
```bash
cargo run
```
## License
MIT
