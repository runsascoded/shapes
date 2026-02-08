# SHA Version Validation for Client-Server Sync

## Summary

Add a version handshake mechanism so the `@apvd/client` WebSocket transport can verify it's talking to a compatible server build.

## Motivation

When using `cargo install --git --rev <SHA>` to install the native server, the JS client (from npm-dist branch) and Rust server may get out of sync. A version mismatch could cause subtle bugs from protocol/type mismatches.

## Proposed Solution

### Server: Add `getVersion` RPC method

```rust
// In handle_json_rpc
"getVersion" => JsonRpcResponse {
    id: req.id.clone(),
    result: Some(serde_json::json!({
        "sha": env!("APVD_BUILD_SHA"),  // Set at build time
        "version": env!("CARGO_PKG_VERSION"),
    })),
    error: None,
}
```

Build SHA should be injected via `build.rs` or env var at compile time.

### Client: Version check on connect

```typescript
// In WebSocketTrainingClient
async ensureConnected(): Promise<void> {
  // ... existing connection logic ...

  // After connected, verify version
  const serverVersion = await this.sendRequest<VersionInfo>("getVersion", {});
  if (this.expectedSha && serverVersion.sha !== this.expectedSha) {
    console.warn(`Version mismatch: client=${this.expectedSha}, server=${serverVersion.sha}`);
    // Optionally throw or emit warning event
  }
}
```

### Configuration

- Client should accept optional `expectedSha` in config
- Mismatch behavior should be configurable: `"warn"` | `"error"` | `"ignore"`
- Default to `"warn"` in dev, could be `"ignore"` in prod if desired

## Non-goals

- No per-request validation (too much overhead)
- No automatic version negotiation or backwards compatibility layer (yet)

## Open Questions

1. Should the npm-dist build inject the SHA into the client bundle automatically?
2. Should there be a protocol version number separate from git SHA for stable API contracts?
