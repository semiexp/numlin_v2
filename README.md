# numlin_v2
The 2nd edition of a Numberlink solver

U-shaped detours one cell deep and of any length are canonicalized in all four
orientations. If every interior vertex of the straight shortcut is clue-free and
confirmed unused, a complete U is rejected, or its last undecided edge is
forbidden. Checks inspect only runs adjacent to changed edges or vertices. The
incident-edge counts used to recognize unused vertices are updated on decisions
and undo, and checked against a full scan in verification builds.

Cut capacities are updated incrementally when edges or path endpoints change,
including during backtracking. Saturated straight cuts visit only undecided edges
using packed bit sets.

The solver also contracts unfinished paths into vertices of capacity one. For
prefixes and suffixes in row-major and column-major order, it checks that the
boundary can carry every clue pair separated by it. Saturation forbids edges
within the boundary and forces unique connections to either side. This is a
necessary-condition check over those separators.

Graph representatives, neighbor extrema, and boundary capacities are cached.
Recursive decisions and undo only mark changed endpoints; the next cut check
updates those vertices and their neighbors. Duplicate partitions between inactive
vertices are skipped, and only saturated boundaries with positive demand are
stored for propagation.

Unit tests compare the straight-cut values against the original full-board
calculation, and independently rebuild the contracted graph and enumerate every
vertex boundary to verify its capacities and demands. To enable these expensive
checks in the reference-solver and fuzz integration tests, run:

```sh
cargo test --features verify-cut-capacity
```

The verification code is excluded from normal builds (including debug builds).
For performance measurements, leave the feature disabled:

```sh
cargo run --release --bin example_tasks
```
