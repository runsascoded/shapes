#!/usr/bin/env python3
"""
Merge platform-specific expected value CSVs into a single file.

For each numeric value, finds the longest decimal representation that both
platforms would round to, preserving only the precision where they agree.
"""

import csv
import sys
from decimal import Decimal, ROUND_HALF_EVEN
from pathlib import Path


def round_to_digits(value: str, digits: int) -> str:
    """Round a numeric string to specified number of significant digits after decimal."""
    if digits <= 0:
        raise ValueError(f"digits must be positive, got {digits}")

    d = Decimal(value)
    sign = '-' if d < 0 else ''
    d = abs(d)

    # Find position of decimal point and first significant digit
    str_val = str(d)
    if '.' in str_val:
        int_part, frac_part = str_val.split('.')
    else:
        int_part, frac_part = str_val, ''

    # For values like 0.00123, we need to count leading zeros in fraction
    if int_part == '0':
        leading_zeros = len(frac_part) - len(frac_part.lstrip('0'))
        # Round to `digits` significant figures
        quantize_exp = Decimal(10) ** (-(leading_zeros + digits))
    else:
        # For values >= 1, round to `digits` decimal places
        quantize_exp = Decimal(10) ** (-digits)

    rounded = d.quantize(quantize_exp, rounding=ROUND_HALF_EVEN)
    return sign + str(rounded)


def find_common_precision(a: str, b: str, min_sig_figs: int = 6) -> str:
    """
    Find the longest decimal string that both values round to.

    Returns the longest string S such that:
    - round(a, precision_of(S)) == S
    - round(b, precision_of(S)) == S

    Args:
        a: First numeric string
        b: Second numeric string
        min_sig_figs: Minimum significant figures to keep (default 6)

    Returns:
        The merged value string
    """
    # Handle non-numeric values (headers, etc.)
    try:
        Decimal(a)
        Decimal(b)
    except:
        if a != b:
            raise ValueError(f"Non-numeric values don't match: {a!r} vs {b!r}")
        return a

    # If they're exactly equal, return as-is
    if a == b:
        return a

    # Find the decimal places in each
    def decimal_places(s: str) -> int:
        if '.' not in s:
            return 0
        return len(s.split('.')[1])

    max_places = max(decimal_places(a), decimal_places(b))

    # Try progressively fewer decimal places until both round to the same value
    for places in range(max_places, 0, -1):
        rounded_a = round_to_digits(a, places)
        rounded_b = round_to_digits(b, places)
        if rounded_a == rounded_b:
            # Verify we have enough significant figures
            sig_figs = len(rounded_a.replace('-', '').replace('.', '').lstrip('0'))
            if sig_figs >= min_sig_figs:
                return rounded_a

    # If we can't find agreement, raise an error
    raise ValueError(
        f"Values diverge too much to merge with {min_sig_figs} sig figs:\n"
        f"  a = {a}\n"
        f"  b = {b}"
    )


def merge_csvs(csv_a_path: Path, csv_b_path: Path, output_path: Path, skip_first_row_merge: bool = True):
    """
    Merge two CSVs, finding common precision for each numeric cell.

    Args:
        csv_a_path: First CSV (e.g., from Ubuntu)
        csv_b_path: Second CSV (e.g., from macOS)
        output_path: Where to write merged CSV
        skip_first_row_merge: If True, data row 0 (after header) is passed through
                              without merging (for fixed initial values)
    """
    with open(csv_a_path) as f_a, open(csv_b_path) as f_b:
        reader_a = csv.reader(f_a)
        reader_b = csv.reader(f_b)

        rows_a = list(reader_a)
        rows_b = list(reader_b)

    # Strict shape validation
    if len(rows_a) != len(rows_b):
        raise ValueError(
            f"Row count mismatch: {csv_a_path} has {len(rows_a)} rows, "
            f"{csv_b_path} has {len(rows_b)} rows"
        )

    if not rows_a:
        raise ValueError("CSVs are empty")

    # Validate header
    header_a, header_b = rows_a[0], rows_b[0]
    if header_a != header_b:
        raise ValueError(
            f"Header mismatch:\n"
            f"  {csv_a_path}: {header_a}\n"
            f"  {csv_b_path}: {header_b}"
        )

    merged_rows = [header_a]  # Keep header as-is

    for row_idx, (row_a, row_b) in enumerate(zip(rows_a[1:], rows_b[1:]), start=1):
        if len(row_a) != len(row_b):
            raise ValueError(
                f"Column count mismatch at row {row_idx}: "
                f"{len(row_a)} vs {len(row_b)}"
            )

        if len(row_a) != len(header_a):
            raise ValueError(
                f"Row {row_idx} has {len(row_a)} columns, header has {len(header_a)}"
            )

        if skip_first_row_merge and row_idx == 1:
            # First data row: initial values, pass through from first file
            merged_rows.append(row_a)
            continue

        merged_row = []
        for col_idx, (val_a, val_b) in enumerate(zip(row_a, row_b)):
            try:
                merged_val = find_common_precision(val_a, val_b)
                merged_row.append(merged_val)
            except ValueError as e:
                raise ValueError(
                    f"Merge failed at row {row_idx}, column {col_idx} ({header_a[col_idx]}):\n{e}"
                ) from e

        merged_rows.append(merged_row)

    # Write output
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerows(merged_rows)

    print(f"Merged {len(merged_rows) - 1} data rows -> {output_path}")


def main():
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <csv_a> <csv_b> <output>", file=sys.stderr)
        print(f"       {sys.argv[0]} --batch <ubuntu_dir> <macos_dir> <output_dir>", file=sys.stderr)
        sys.exit(1)

    if sys.argv[1] == '--batch':
        if len(sys.argv) != 5:
            print("--batch requires: <ubuntu_dir> <macos_dir> <output_dir>", file=sys.stderr)
            sys.exit(1)

        ubuntu_dir = Path(sys.argv[2])
        macos_dir = Path(sys.argv[3])
        output_dir = Path(sys.argv[4])

        # Find all test directories
        ubuntu_csvs = sorted(ubuntu_dir.glob('*/linux.csv'))

        for ubuntu_csv in ubuntu_csvs:
            test_name = ubuntu_csv.parent.name
            macos_csv = macos_dir / test_name / 'macos.csv'

            if not macos_csv.exists():
                print(f"Warning: No macOS CSV for {test_name}, skipping", file=sys.stderr)
                continue

            output_csv = output_dir / test_name / 'expected.csv'
            merge_csvs(ubuntu_csv, macos_csv, output_csv)
    else:
        csv_a = Path(sys.argv[1])
        csv_b = Path(sys.argv[2])
        output = Path(sys.argv[3])
        merge_csvs(csv_a, csv_b, output)


if __name__ == '__main__':
    main()
