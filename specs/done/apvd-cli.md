# apvd CLI Specification

## Overview

Two CLI binaries that share the same interface:
- **`apvd`** - Native Rust binary (fast, for production use)
- **`apvd-wasm`** - WASM binary running in Node.js (for browser parity testing)

Both use `apvd-core` for all logic. `apvd-wasm` produces **bit-identical** results to WASM running in browser Worker - same `.wasm` binary, same wasm-bindgen glue, same V8 engine.

## Architecture

```
shapes/
├── apvd-core/          # Shared library (math, optimization, trace, BTD)
├── apvd-wasm/          # WASM build (used by browser AND apvd-wasm CLI)
├── apvd/               # Native CLI binary
│   ├── Cargo.toml
│   └── src/main.rs
└── apvd-wasm-cli/      # Node.js wrapper for WASM CLI
    ├── package.json
    └── src/cli.ts      # Thin wrapper calling apvd-wasm
```

The `apvd-wasm` CLI is a Node.js script that loads the same `apvd_wasm.wasm` binary used by the browser, ensuring exact parity.

## Command Structure

```
apvd <command> [subcommand] [options]

Commands:
  train     Run gradient descent training
  trace     Trace file operations (info, convert, diff, verify, etc.)
  parity    Compare native vs WASM execution
```

## Training Commands

### `apvd train`

Run training from command line.

```bash
# Train from JSON config
apvd train config.json -o trace.json --steps 10000

# Train with inline config
apvd train --circles 3 --targets "16,8,4,2,1" -o trace.json --steps 10000

# Resume from existing trace
apvd train trace.json --resume --steps 5000 -o extended.json

# With specific tiering for output
apvd train config.json -o trace.json --steps 10000 \
    --max-btd 500 --interval 1000
```

**Options:**
- `-o, --output <file>` - Output trace file (required)
- `--steps <n>` - Number of training steps (default: 10000)
- `--learning-rate <lr>` - Learning rate (default: 0.1)
- `--threshold <eps>` - Convergence threshold (default: 1e-10)
- `--seed <n>` - Random seed for reproducibility
- `--resume` - Continue from input trace
- `--progress` - Show progress bar
- `--max-btd <n>` - BTD keyframes to retain in output
- `--interval <n>` - Interval keyframe spacing in output

## Trace Commands

### `apvd trace info <file>`

Display trace metadata and statistics.

```bash
$ apvd trace info trace.json

Trace: trace.json
  Version: 2
  Created: 2026-02-01T14:30:52Z
  Total steps: 10000
  Min error: 2.3e-9 (step 8745)

Shapes:
  3 ellipses (XYRRT)
  15 variables total

Keyframes:
  BTD keyframes: 156
  Interval keyframes: 10
  Total stored: 166

Tiering:
  Strategy: btd-evenly-spaced
  Max BTD: 1000
  Interval spacing: 1000

Size:
  Uncompressed: 45.2 KB
  Gzipped: 12.1 KB

Recomputation:
  Max distance: 423 steps
  Avg distance: 87 steps
```

**Options:**
- `--json` - Output as JSON
- `--verbose` - Include per-keyframe details

### `apvd trace convert <input> -o <output>`

Convert trace between tiering configurations.

```bash
# Reduce to 100 BTD keyframes, 2000-step intervals
apvd trace convert trace.json -o compact.json \
    --max-btd 100 --interval 2000

# Disable interval keyframes (BTD-only)
apvd trace convert trace.json -o btd-only.json \
    --max-btd 500 --interval 0

# Upsample intervals (requires recomputation)
apvd trace convert trace.json -o fine.json --interval 100

# Convert v1 format to v2
apvd trace convert legacy.json -o modern.json --upgrade

# Strip error array from v1 trace
apvd trace convert trace.json -o slim.json --strip-errors
```

**Options:**
- `-o, --output <file>` - Output file (required)
- `--max-btd <n>` - Maximum BTD keyframes
- `--interval <n>` - Interval keyframe spacing (0 to disable)
- `--compress` - Output as .json.gz
- `--upgrade` - Convert v1 to v2 format
- `--strip-errors` - Remove dense error array
- `--include-errors` - Add dense error array
- `--force` - Overwrite existing output

### `apvd trace diff <file1> <file2>`

Compare two traces.

```bash
$ apvd trace diff trace1.json trace2.json

Comparing traces...

Config:
  Inputs: identical
  Targets: identical
  Learning rate: 0.1 vs 0.1 (identical)

Steps:
  trace1.json: 10000 steps
  trace2.json: 10000 steps

Error convergence:
  trace1.json: 2.3e-9 at step 8745
  trace2.json: 2.3e-9 at step 8745
  Difference: 1.2e-15 (within tolerance)

Shape divergence at step 5000:
  Max coordinate diff: 3.4e-12
  Error diff: 1.1e-14
```

**Options:**
- `--step <n>` - Compare at specific step
- `--tolerance <eps>` - Difference tolerance (default: 1e-10)
- `--all-steps` - Compare every step (slow)
- `--json` - Output as JSON

### `apvd trace verify <file>`

Verify trace integrity and reconstruction accuracy.

```bash
$ apvd trace verify trace.json

Verifying trace.json...
  ✓ JSON schema valid
  ✓ Keyframes sorted by step index
  ✓ BTD keyframes monotonically decreasing in error
  ✓ Step 0 present
  ✓ Final step present

Reconstruction verification (100 samples):
  ✓ All samples within tolerance (1e-10)

Trace verified successfully.
```

**Options:**
- `--tolerance <eps>` - Reconstruction tolerance (default: 1e-10)
- `--samples <n>` - Random steps to verify (default: 100)
- `--exhaustive` - Verify every step
- `--quick` - Schema only, skip reconstruction

### `apvd trace benchmark <file>`

Benchmark recomputation performance.

```bash
$ apvd trace benchmark trace.json

Random access (1000 samples):
  Min: 0.02ms (keyframe hit)
  Max: 8.4ms (423 steps from keyframe)
  Avg: 1.7ms
  P50: 1.2ms
  P95: 5.1ms

Sequential scan (step 0 → 10000):
  Total: 2.3s
  Per step: 0.23ms
```

**Options:**
- `--samples <n>` - Random access samples (default: 1000)
- `--sequential` - Include sequential scan benchmark
- `--json` - Output as JSON

### `apvd trace reconstruct <file> <step>`

Reconstruct and output shapes at a specific step.

```bash
$ apvd trace reconstruct trace.json 5000

Step 5000 (reconstructed from keyframe at 4096):
  Recomputation: 904 steps, 1.8ms

Shapes:
  [0] XYRRT: center=(0.123, 0.456), radii=(0.789, 0.321), theta=0.543
  [1] XYRRT: center=(-0.234, 0.567), radii=(0.654, 0.432), theta=1.234
  [2] XYRRT: center=(0.345, -0.678), radii=(0.567, 0.345), theta=2.345

Error: 4.567e-6
```

**Options:**
- `--json` - Output as JSON
- `--svg <file>` - Render to SVG
- `--range <start>:<end>` - Reconstruct range of steps

## Parity Commands

### `apvd parity test`

Verify native and WASM produce identical results.

```bash
$ apvd parity test --steps 1000 --seed 42

Running parity test...
  Config: 3 circles, seed 42, 1000 steps

Native execution:
  Time: 0.8s
  Final error: 2.345678901234e-7

WASM execution:
  Time: 2.1s
  Final error: 2.345678901234e-7

Step-by-step comparison:
  Steps 0-100: bit-identical ✓
  Steps 100-500: max diff 1.2e-15 ✓
  Steps 500-1000: max diff 3.4e-14 ✓

Parity: PASS (all differences within floating-point tolerance)
```

**Options:**
- `--steps <n>` - Steps to compare (default: 1000)
- `--seed <n>` - Random seed (default: random)
- `--config <file>` - Use specific config
- `--tolerance <eps>` - Max allowed difference (default: 1e-10)
- `--verbose` - Show per-step differences
- `--stop-on-divergence` - Stop at first difference > tolerance

### `apvd parity train`

Run training with both native and WASM, save both traces.

```bash
$ apvd parity train config.json --steps 10000 \
    --native-output native.json \
    --wasm-output wasm.json

Training with native...
  10000 steps in 8.2s (1220 steps/sec)
  Final error: 2.3e-9

Training with WASM...
  10000 steps in 21.4s (467 steps/sec)
  Final error: 2.3e-9

Traces saved:
  Native: native.json (45 KB)
  WASM: wasm.json (45 KB)

Quick diff: identical to 1e-14
```

## WASM CLI Implementation

The `apvd-wasm` binary is a Node.js script:

```typescript
// apvd-wasm-cli/src/cli.ts
import { initWasm, train, reconstructStep } from 'apvd-wasm';

async function main() {
  await initWasm();

  // Parse args and call same functions as browser Worker
  const args = parseArgs(process.argv);

  switch (args.command) {
    case 'train':
      return runTrain(args);
    case 'trace':
      return runTrace(args);
    // ... same interface as native apvd
  }
}
```

**package.json:**
```json
{
  "name": "apvd-wasm-cli",
  "bin": {
    "apvd-wasm": "./dist/cli.js"
  },
  "dependencies": {
    "apvd-wasm": "workspace:*"
  }
}
```

This ensures `apvd-wasm` loads the exact same `.wasm` binary as the browser, providing true parity testing.

## Native CLI Implementation

### Cargo.toml

```toml
[package]
name = "apvd"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "apvd"
path = "src/main.rs"

[dependencies]
apvd-core = { path = "../apvd-core" }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
flate2 = "1"
indicatif = "0.17"
colored = "2"
anyhow = "1"
rand = "0.8"
```

### Main Structure

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "apvd")]
#[command(about = "Area-Proportional Venn Diagrams")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run training
    Train(TrainArgs),

    /// Trace file operations
    Trace {
        #[command(subcommand)]
        command: TraceCommands,
    },

    /// Native/WASM parity testing
    Parity {
        #[command(subcommand)]
        command: ParityCommands,
    },
}

#[derive(Subcommand)]
enum TraceCommands {
    Info(TraceInfoArgs),
    Convert(TraceConvertArgs),
    Diff(TraceDiffArgs),
    Verify(TraceVerifyArgs),
    Benchmark(TraceBenchmarkArgs),
    Reconstruct(TraceReconstructArgs),
}

#[derive(Subcommand)]
enum ParityCommands {
    Test(ParityTestArgs),
    Train(ParityTrainArgs),
}
```

## Testing

### Parity Test Suite

```rust
#[cfg(test)]
mod parity_tests {
    /// Verify native matches WASM for various scenarios
    #[test]
    fn test_3_circles_1000_steps() {
        let config = Config::circles(3);
        let native = train_native(&config, 1000, Some(42));
        let wasm = train_wasm(&config, 1000, Some(42));

        assert_traces_equal(&native, &wasm, 1e-10);
    }

    #[test]
    fn test_3_ellipses_10000_steps() {
        let config = Config::ellipses(3);
        let native = train_native(&config, 10000, Some(123));
        let wasm = train_wasm(&config, 10000, Some(123));

        // May have small divergence at high step counts
        assert_traces_equal(&native, &wasm, 1e-8);
    }
}
```

### CI Integration

```yaml
# .github/workflows/parity.yml
parity-test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions-rs/toolchain@v1
    - uses: pnpm/action-setup@v2

    - name: Build native
      run: cargo build --release -p apvd

    - name: Build WASM
      run: wasm-pack build apvd-wasm --target nodejs

    - name: Build WASM CLI
      run: cd apvd-wasm-cli && pnpm install && pnpm build

    - name: Run parity tests
      run: |
        ./target/release/apvd parity test --steps 10000 --seed 42
        ./target/release/apvd parity test --steps 10000 --seed 123
        ./target/release/apvd parity test --steps 10000 --seed 456
```

## Usage Examples

### Workflow: Verify Browser Behavior Offline

```bash
# Export trace from browser (Downloads folder)
# Then verify it matches what CLI would produce

# Re-run same training with CLI
apvd-wasm train --seed 42 --steps 8201 \
    --circles 3 --targets "16,8,4,2,1" \
    -o cli-trace.json

# Compare to browser export
apvd trace diff ~/Downloads/browser-trace.json cli-trace.json
# Should show: "identical" or very small differences
```

### Workflow: Benchmark Tiering Tradeoffs

```bash
# Generate trace
apvd train config.json -o full.json --steps 50000

# Try different tiering levels
for btd in 100 500 1000 2000; do
  apvd trace convert full.json -o "btd-$btd.json" --max-btd $btd --interval 1000
  echo "=== BTD $btd ==="
  apvd trace info "btd-$btd.json" | grep -E "Size|Max distance"
  apvd trace benchmark "btd-$btd.json" | grep "Avg"
done
```

### Workflow: Reproduce Browser Issue

```bash
# User reports bug with specific config
# Reproduce exactly with WASM CLI

apvd-wasm train bug-config.json --steps 5000 -o repro.json
apvd trace info repro.json
apvd trace reconstruct repro.json 3456 --json
```
