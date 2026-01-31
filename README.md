# shapes

Rust library for computing differentiable shape intersections with automatic differentiation. Used by [apvd](https://github.com/runsascoded/apvd) for area-proportional Venn diagram generation.

## Features

- **Automatic differentiation**: Compute gradients of region areas w.r.t. shape parameters
- **Multiple shape types**: Circles, axis-aligned ellipses (XYRR), rotated ellipses (XYRRT), polygons
- **WASM support**: Run in the browser via WebAssembly
- **Native CLI**: Faster training with parallel scene optimization
- **WebSocket server**: Real-time training updates for frontends

## Workspace Structure

```
shapes/
├── apvd-core/     # Core computation library (platform-agnostic)
├── apvd-wasm/     # WASM bindings + WorkerTrainingClient for browser
├── apvd-cli/      # Native CLI and WebSocket server
└── client/        # @apvd/client - TypeScript types + WebSocketTrainingClient
```

## Installation

### WASM (Browser)

```bash
cd apvd-wasm && wasm-pack build --target web
```

Output: `apvd-wasm/pkg/`

### TypeScript Client

```bash
# Build @apvd/client (types + WebSocketTrainingClient)
cd client && pnpm build

# Build apvd-wasm TypeScript (WorkerTrainingClient)
cd apvd-wasm/ts && pnpm build
```

### Native CLI

```bash
cargo build -p apvd-cli --release
```

Output: `target/release/apvd`

## Usage

### TypeScript Client API

Two packages provide a unified `TrainingClient` interface:

- **`@apvd/client`**: Types + `WebSocketTrainingClient` (connects to `apvd serve`)
- **`apvd-wasm`**: WASM + `WorkerTrainingClient` (runs training in Web Worker)

```typescript
// WebSocket transport (native Rust server)
import { createTrainingClient } from "@apvd/client";

const client = createTrainingClient({
  transport: "websocket",
  url: "ws://localhost:8080"
});

// Worker transport (WASM in browser) - use apvd-wasm package
import { createWorkerTrainingClient } from "apvd-wasm/client";

const client = createWorkerTrainingClient();

// Both clients share the same API
const handle = await client.startTraining({
  inputs: [
    [{ kind: "Circle", c: { x: 0, y: 0 }, r: 1 }, [true, true, true]],
    [{ kind: "Circle", c: { x: 1, y: 0 }, r: 1 }, [true, true, true]],
  ],
  targets: { "0*": 3, "*1": 5, "01": 1 },
});

client.onProgress((update) => {
  console.log(`Step ${update.currentStep}, error: ${update.error}`);
});
```

### Low-level WASM API

```javascript
import init, { make_model, train_robust, step, is_converged } from 'apvd-wasm';

await init();

// Define shapes with trainable parameters
const inputs = [
  [{ kind: "Circle", c: { x: 0, y: 0 }, r: 1 }, [[1], [1], [1]]],  // cx, cy, r trainable
  [{ kind: "Circle", c: { x: 1, y: 0 }, r: 1 }, [[1], [0], [1]]],  // cx, r trainable (cy fixed)
];

// Target area fractions
const targets = { "10": 0.3, "01": 0.3, "11": 0.4 };

// Train
const model = make_model(inputs, targets);
const trained = train_robust(model, 1000);

console.log(`Final error: ${trained.steps.at(-1).error.v}`);
```

### CLI

```bash
# Batch training
apvd train -s shapes.json -t targets.json -m 1000 -p 6

# WebSocket server for frontend
apvd serve -p 8080
```

### WebSocket Protocol

Connect to `ws://host:port/ws`:

```javascript
// Start training
ws.send(JSON.stringify({
  type: "StartTraining",
  shapes: [...],
  targets: {...},
  max_steps: 1000
}));

// Receive updates
ws.onmessage = (e) => {
  const msg = JSON.parse(e.data);
  if (msg.type === "StepUpdate") {
    console.log(`Step ${msg.step_idx}: error=${msg.error}`);
  }
};
```

## How It Works

1. **Scene analysis**: Find all intersection points between shapes (quartic solver for ellipses)
2. **Region computation**: Build boundary graph and compute signed area of each region
3. **Gradient descent**: Autodiff computes error gradients; optimizer updates shape parameters
4. **Convergence**: Repeat until error threshold reached or max steps

## Development

```bash
# Run tests
cargo test

# Run specific test
cargo test -p apvd-core fizz_buzz_bazz

# With logging
RUST_LOG=debug cargo test
```

## License

MIT
