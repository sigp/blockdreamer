`blockdreamer`
=============

Blockdreamer is a microservice for triggering block production on Ethereum consensus clients.

Each slot it hits the `/eth/v2/validator/blocks/slot` endpoint for each node in its `config.toml`.

This is useful for several tasks:

- Benchmarking/stress testing
- Generating training data for [blockprint](https://github.com/sigp/blockprint)
- Computing similarity scores for blocks (an alternative to blockprint, see `src/distance.rs`)

## Installation

A Docker image is available on the GitHub container registry:

```
docker pull ghcr.io/sigp/blockdreamer
```

Or you can build from source:

```
cargo build --release
```

## Configuration

Blockdreamer is configured by a `config.toml` provided to the `--config` flag.

```
Ethereum block hallucinator.

Usage: blockdreamer --config <PATH>

Options:
      --config <PATH>  Path to a TOML configuration file. See docs for examples
  -h, --help           Print help
  -V, --version        Print version
```

An example configuration file can be found at [`example.toml`](./example.toml).

A full list of configuration options can be found in the source: [`src/config.rs`](./src/config.rs).

## Consensus Node Configuration

Ensure that all the consensus nodes configured with blockdreamer have a fee recipient set.
Block proposals may fail if the fee recipient is not set. A dummy value is usually sufficient,
e.g. for Lighthouse:

```
lighthouse bn \
  --suggested-fee-recipient 0xffffffffffffffffffffffffffffffffffffffff
```

## Execution Node Configuration

Blockdreamer is often used in conjunction with [Eleel][], which both simplifies the maintenance
of the consensus node swarm and streamlines block building. Unlike execution nodes which may take
significant time to build an execution payload, Eleel can build a dummy payload in milliseconds,
and isn't fussy about how the consensus node asks for that payload.

[Eleel]: https://github.com/sigp/eleel
