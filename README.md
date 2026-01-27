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
├── apvd-wasm/     # WASM bindings for browser
└── apvd-cli/      # Native CLI and WebSocket server
```

## Installation

### WASM (Browser)

```bash
cd apvd-wasm && wasm-pack build --target web
```

Output: `apvd-wasm/pkg/`

### Native CLI

```bash
cargo build -p apvd-cli --release
```

Output: `target/release/apvd`

## Usage

### WASM API

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
