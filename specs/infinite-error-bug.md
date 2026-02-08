# Infinite Error Bug in createModel

## Summary

The `createModel` RPC returns `error: inf` (serialized as `null` in JSON) for valid inputs, causing the frontend to display "Infinity" error and preventing training from starting.

## Reproduction

1. Start server: `cargo run --bin apvd serve`
2. Load frontend with 3-shape Venn diagram: `http://localhost:5184/apvd/#t=i35,21,7,15,5,3,1`
3. Observe console: `createModel: computed error = inf`

Server log shows:
```
createModel: received 3 inputs, targets: {"---": -57.0, "--2": 8.0, "-1-": 12.0, "-12": 2.0, "0--": 24.0, "0-2": 4.0, "01-": 6.0, "012": 1.0}
createModel: computed error = inf, is_nan = false
```

## Suspected Causes

1. **Negative target for "---" region**: The exclusive targets include `"---": -57.0` (outside all shapes). This negative value may cause division issues or is mathematically invalid.

2. **Error calculation edge case**: The error computation in `Step::new` or related functions may divide by zero or produce infinity for certain input configurations.

3. **Inclusion-exclusion math**: The frontend's `expandTargets` function computes exclusive from inclusive targets. For `"---"`:
   ```
   --- = *** - 0** - *1* + 01* - **2 + 0*2 + *12 - 012
   ```
   This can legitimately be negative if shapes cover more than the total target area.

## Investigation Steps

1. Add debug logging to `Step::new` to trace where infinity is introduced
2. Check if `Targets::new` handles negative values correctly
3. Verify the error formula in `step.rs` - look for divisions that could produce inf
4. Test with simpler inputs (1 or 2 shapes) to isolate the issue

## Possible Fixes

1. **Filter "---" target**: The "none" region (outside all shapes) shouldn't contribute to error - it's not a meaningful constraint
2. **Clamp negative targets to zero**: Negative exclusive targets are mathematically possible but may need special handling
3. **Guard divisions**: Add checks for zero denominators in error calculation
4. **Return error as Result**: Instead of silently producing inf, return a descriptive error

## Files to Investigate

- `apvd-core/src/optimization/step.rs` - Step::new, error calculation
- `apvd-core/src/optimization/targets.rs` - Targets::new, disjoints
- `apvd-cli/src/server.rs` - handle_create_model

## Impact

- Frontend shows "Infinity" error instead of actual value
- `converged` check (`error < threshold`) is always false
- Training cannot detect convergence properly
