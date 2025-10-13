---
title: "Installation"
weight: 1
---

Reqs can be installed in several ways, depending on your preference and environment.

### From Cargo

If you have Rust and Cargo installed, you can install Reqs directly from [crates.io](https://crates.io/crates/reqs):

```bash
cargo install reqs
```

### From Source

To build Reqs from source, you'll need to have Rust and Cargo installed.

```bash
git clone https://github.com/hahwul/reqs.git
cd reqs
cargo build --release
```

The compiled binary will be located at `target/release/reqs`.

### From Docker

Reqs is also available as a Docker image on GitHub Container Registry.

You can pull the image using the following command:

```bash
docker pull ghcr.io/hahwul/reqs:latest
```

To run Reqs with Docker:

```bash
# Using stdin
echo "https://example.com" | docker run -i ghcr.io/hahwul/reqs:latest

# With options
cat urls.txt | docker run -i ghcr.io/hahwul/reqs:latest --timeout 5 --format jsonl
```

### Verifying Installation

After installation, verify that Reqs is working correctly:

```bash
reqs --version
```
