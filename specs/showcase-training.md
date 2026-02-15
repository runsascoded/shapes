# Showcase Training Runs

Run dodecagon (12-vertex polygon) training at 30k steps on all test cases. These need more compute than a laptop — run sequentially or 2-3 at a time to avoid OOM.

## Build

```bash
cargo build -p apvd-cli --release
```

## Commands

Run each with `-R` (robust optimizer: Adam + gradient clipping + backtracking), `-m 30000` (30k steps), `-p 12` (12 shape permutation variants in parallel), `-T` (tiered keyframes for seekable traces):

### 4-set dodecagons (12 variants each)

```bash
target/release/apvd train -c testcases/variant-callers-dodecagons.json -R -m 30000 -p 12 -T -o testcases/variant-callers-dodecagons.trace.json
target/release/apvd train -c testcases/zhang2014d-dodecagons.json -R -m 30000 -p 12 -T -o testcases/zhang2014d-dodecagons.trace.json
target/release/apvd train -c testcases/zhang2014e-dodecagons.json -R -m 30000 -p 12 -T -o testcases/zhang2014e-dodecagons.trace.json
target/release/apvd train -c testcases/zhang2014f-dodecagons.json -R -m 30000 -p 12 -T -o testcases/zhang2014f-dodecagons.trace.json
```

### 5-set dodecagons (6 variants each)

```bash
target/release/apvd train -c testcases/5set-example1-dodecagons.json -R -m 30000 -p 6 -T -o testcases/5set-example1-dodecagons.trace.json
target/release/apvd train -c testcases/5set-example2-dodecagons.json -R -m 30000 -p 6 -T -o testcases/5set-example2-dodecagons.trace.json
```

## Baselines (10k steps, from laptop runs)

| Dataset | Ellipses | Octagons (8v) | Notes |
|---------|----------|---------------|-------|
| variant-callers | **1.4%** | 3.7% | Ellipses won here |
| zhang2014d | 18.2% | **8.6%** | Octagons much better |
| zhang2014e | 9.2% | **7.8%** | Octagons slightly better |
| zhang2014f | 6.1% | **1.1%** | Octagons much better |
| 5set-example1 | - | 6.7% | |
| 5set-example2 | - | 12.9% | Struggled |

With dodecagons (12v, 24 DOF/shape) and 3× more steps, we expect improvement over the octagon baselines.

## After Training

The `-T` flag produces tiered-keyframe traces that are seekable in the frontend. The trace `.json` files can be uploaded via the ⤒ icon in the static FE at `~/c/rac/apvd/static`.

Add `-z` to also gzip-compress the trace output (produces `.json.gz`).
