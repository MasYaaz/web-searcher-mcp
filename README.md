# Web Searcher

A small MCP server for searching the web via DuckDuckGo HTML and extracting web page content as clean Markdown.

This server runs over stdio, so it can be used by MCP clients such as Claude Desktop, Cursor, or any client that supports the mcpServers configuration.

## Features

- `web_search`: Search the web using DuckDuckGo HTML.
- `fetch_web_content`: Read a public URL and convert its main content to Markdown.
- Search time filters: `day`, `week`, `month`, or `year`.
- DuckDuckGo region filter, default is `id-id`.
- SSRF guard to block local/private URLs when fetching content.

## Requirements

- Rust toolchain with `cargo`
- Internet connection for search/fetch

## Installation

Build and install to `~/.mcp/web-searcher`:

```bash
./install.sh
```

Or build manually:

```bash
cargo build --release
```

The resulting binary will be at:

```text
target/release/web-searcher
```

## MCP Configuration

Example MCP client configuration:

```json
{
  "mcpServers": {
    "web-agent-mcp": {
      "command": "/home/.mcp/web-searcher"
    }
  }
}
```

Replace `command` with the binary path on your machine.

## Tools

### `web_search`

Search for information on DuckDuckGo.

Input:

```json
{
  "query": "rust mcp server",
  "max_results": 10,
  "time_range": "week",
  "region": "id-id"
}
```

Parameters:

| Name          | Required | Default | Description                                    |
| ------------- | -------- | ------- | ---------------------------------------------- |
| `query`       | Yes      | -       | Search keywords                                |
| `max_results` | No       | `15`    | Maximum number of results                      |
| `time_range`  | No       | -       | `day`, `week`, `month`, or `year`              |
| `region`      | No       | `id-id` | DuckDuckGo region code, e.g., `id-id`, `us-en` |

### `fetch_web_content`

Fetch a public web page and return its content as Markdown.

Input:

```json
{
  "url": "https://example.com/article"
}
```

Parameters:

| Name  | Required | Default | Description                |
| ----- | -------- | ------- | -------------------------- |
| `url` | Yes      | -       | Full `http` or `https` URL |

## Development

```bash
cargo check
cargo build
cargo run
```

The server reads JSON-RPC line by line from stdin and writes the JSON-RPC response to stdout.

---
