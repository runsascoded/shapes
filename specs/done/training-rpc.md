# Training RPC Implementation

## Summary

Add JSON-RPC `train` method to the WebSocket server to enable step-by-step training from the frontend.

## Current State

- `createModel` RPC works - creates step 0 with geometry
- Legacy `StartTraining` exists but uses tag-based protocol
- Client (`@apvd/client`) sends `"train"` JSON-RPC which server doesn't handle

## Required Changes

### Server (apvd-cli/src/server.rs)

Add `"train"` handler to `handle_json_rpc`:

```rust
"train" => handle_train(&req.id, &req.params, config),
```

The `train` method should:
1. Accept `TrainingRequest` params (inputs, targets, max_steps, learning_rate, etc.)
2. Start training in background task
3. Return `TrainingHandle { id: string }`
4. Stream progress updates as JSON-RPC notifications

### Progress Notifications

Use JSON-RPC notification format for streaming updates:
```json
{"method": "progress", "params": {"handleId": "...", "stepIndex": 42, "error": 0.123, ...}}
```

### Additional Methods Needed

- `"getStep"` - Get specific step state
- `"getStepWithGeometry"` - Get step with full geometry
- `"stop"` - Stop training session

## Alternative: Hybrid Approach

Keep legacy protocol for streaming, use JSON-RPC for request/response:
- `createModel` (JSON-RPC) - initial step
- `StartTraining` (legacy) - streaming updates
- `StopTraining` (legacy) - stop

This avoids reimplementing the working legacy streaming logic.
