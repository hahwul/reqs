# Reqs Documentation

This directory contains the documentation for Reqs, built using [Zola](https://www.getzola.org/) static site generator with the [Goyo](https://github.com/hahwul/goyo) theme.

## Prerequisites

- [Zola](https://www.getzola.org/documentation/getting-started/installation/) 0.19.0 or higher

## Building the Documentation

To build the documentation locally:

```bash
cd docs
zola build
```

The built site will be in the `docs/public/` directory.

## Serving Locally

To serve the documentation locally for preview:

```bash
cd docs
zola serve
```

Then open your browser to `http://127.0.0.1:1111/`.

## Structure

- `content/` - Documentation pages in Markdown format
  - `get_started/` - Getting started guides (installation, usage)
  - `mcp_mode/` - MCP server mode documentation
- `static/` - Static assets (images, favicon, etc.)
- `themes/goyo/` - Goyo theme (git submodule)
- `config.toml` - Zola configuration

## Contributing

To add or update documentation:

1. Edit the relevant Markdown files in `content/`
2. Test your changes locally with `zola serve`
3. Submit a pull request

## Deployment

The documentation is automatically deployed to [reqs.hahwul.com](https://reqs.hahwul.com) via GitHub Pages.
