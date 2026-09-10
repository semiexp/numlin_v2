# numlin_v2
The 2nd edition of a Numberlink solver

Paths carrying the same clue may not pass one another without taking an available
straight shortcut. Adjacent vertices with the same clue label must be joined. If
they are farther apart in a row or column and every intervening vertex is confirmed
unused, the branch is rejected. Unknown cells and other paths block this check.
Labels cover internal vertices and completed paths, and are assigned incrementally
when a clue-free path attaches to a clue. Labels are restored on undo, and only
straight runs from changed vertices are rescanned. This canonicalization removes
longer solutions that can be shortened without disturbing any other path.

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
updates their nodes. Neighbors are revisited only when a path representative
changes. Isolated free vertices and duplicate partitions between inactive vertices
are skipped; isolated clues still contribute demand. Propagation scans vertices
only for boundaries that are actually saturated with positive demand.

Vertex degrees are derived from the existing path endpoints and incident-edge
counts, avoiding repeated scans of their four edges. Already decided edges take
a short return path, and decisions stop once a branch is inconsistent. Vertex
coordinates are cached for cut updates and undo.

Unit tests compare the straight-cut values against the original full-board
calculation, and independently rebuild the contracted graph and enumerate every
vertex boundary to verify its capacities and demands. The endpoint-derived degrees
are also compared with the actual incident lines. To enable these expensive
checks in the reference-solver and fuzz integration tests, run:

```sh
cargo test --features verify-cut-capacity
```

Verification also rebuilds path labels from the original clues and line edges,
and enumerates all straight shortcut pairs to check incremental propagation. The
independent CSP solver enforces the shortcut rule using its final line labels.

The verification code is excluded from normal builds (including debug builds).
For performance measurements, leave the feature disabled:

```sh
cargo run --release --bin example_tasks
```
