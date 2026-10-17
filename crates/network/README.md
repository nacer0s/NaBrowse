# nabrowse-network

Networking stack for NaBrowse: HTTP/1.1 client, TLS via rustls, DNS, fetch.

## Features

- HTTP/1.1 GET requests
- TLS 1.2/1.3 via rustls (HTTPS)
- System root certificates (webpki-roots)
- DNS resolution via OS
- Privacy-first: no telemetry, no tracking

## Example

```rust
use nabrowse_network::NetworkClient;

let client = NetworkClient::new().unwrap();
let response = client.fetch("https://example.com").await.unwrap();
println!("Status: {}", response.status);
println!("Body: {}", String::from_utf8_lossy(&response.body));
```

## License

MIT OR Apache-2.0.
