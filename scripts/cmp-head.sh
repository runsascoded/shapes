#!/usr/bin/env bash

final_error() {
    tail -n1 | awk -F, "{ print \$$col }"
}

for f in "$@"; do
    col="$(echo "`head -n1 "$f" | tr , '\n' | wc -l` / 2" | bc)"
    echo "$f (col $col):"
    # diff <(git show "HEAD:$f" | tail -n1 | awk -F, "{ print \$$col }") <(cat "$f" | tail -n1 | awk -F, "{ print \$$col }"
    diff -U0 <(git show "HEAD:$f" | final_error) <(cat "$f" | final_error) | grep -e '^+[0-9]' -e '^-[0-9]'
done
