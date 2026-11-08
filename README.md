# moose-perfgraph

Rank [MOOSE](https://mooseframework.inl.gov) `PerfGraph` sections by self and
total time — for logs, CI artifacts, and remote shells where the raw
`VariadicTable` output is painful to eyeball.

## Why

MOOSE's performance report (`Outputs/perf_graph` or `--timing`) prints three
ASCII tables: the full call tree (`Performance Graph`), the most expensive
path (`Heaviest Branch`), and a flat self-time ranking (`Heaviest Sections`).
This tool parses them and re-ranks, so "what is actually slow in this run?"
is a one-liner.

## Install

```sh
cargo install --git https://github.com/andrewwhitecdw/moose-perfgraph
```

## Usage

```sh
moose-perfgraph run.log
moose-perfgraph run.log --top 10
moose-perfgraph run.log --name jacobian
moose-perfgraph run.log --min-self 0.5
```

Example:

```
performance graph: 42 sections

top by self time:
  Solve: self=1.500s (50.0%)  total=2.900s (96.7%)  calls=1
    Compute Jacobian: self=0.800s (26.7%)  total=0.800s (26.7%)  calls=10
    Compute Residual: self=0.400s (13.3%)  total=0.400s (13.3%)  calls=10
```

Tree indentation is preserved, so nested sections stay in context.

## Notes

The parser reads the `VariadicTable` format directly and tolerates the
column-name variants MOOSE uses (`Self` vs `Self(s)`). It reads plain console
output; it does not need the MOOSE executable.

## Relationship to MOOSE

MOOSE is developed by Idaho National Laboratory and licensed LGPL-2.1. This
tool is an independent MIT-licensed project that reads MOOSE console output;
it contains no MOOSE code.

## License

MIT
