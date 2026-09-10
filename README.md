# numlin_v2
The 2nd edition of a Numberlink solver

Cut capacities are updated incrementally when edges or path endpoints change,
including during backtracking. Unit tests automatically compare these values
against the original full-board calculation. To enable the same checks in the
reference-solver and fuzz integration tests, run:

```sh
cargo test --features verify-cut-capacity
```

The verification code is excluded from normal builds (including debug builds).
For performance measurements, leave the feature disabled:

```sh
cargo run --release --bin example_tasks
```
