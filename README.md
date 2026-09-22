# Web Searcher

MCP server kecil untuk mencari web lewat DuckDuckGo HTML dan mengambil isi halaman web sebagai Markdown bersih.

Server ini berjalan lewat stdio, jadi bisa dipakai oleh MCP client seperti Claude Desktop, Cursor, atau client lain yang mendukung konfigurasi `mcpServers`.

## Fitur

- `web_search`: mencari hasil web dari DuckDuckGo HTML.
- `fetch_web_content`: membaca URL publik dan mengubah konten utamanya ke Markdown.
- Filter waktu pencarian: `day`, `week`, `month`, atau `year`.
- Filter wilayah DuckDuckGo, default `id-id`.
- SSRF guard untuk memblokir URL lokal/private saat fetch konten.
- Filter semantik opsional memakai llama.cpp embedding server.

## Kebutuhan

- Rust toolchain dengan `cargo`
- Koneksi internet untuk pencarian/fetch web
- Opsional: llama.cpp embedding server untuk filter semantik saat memakai parameter `query` di `fetch_web_content`

## Instalasi

Build dan install ke `~/.mcp/rust-mcp/web-searcher`:

```bash
./install.sh
```

Atau build manual:

```bash
cargo build --release
```

Binary hasil build ada di:

```text
target/release/web-agent-mcp
```

## Konfigurasi MCP

Contoh konfigurasi MCP client:

```json
{
  "mcpServers": {
    "web-agent-mcp": {
      "command": "/home/USER/.mcp/web-searcher"
    }
  }
}
```

Ganti `command` dengan path binary di mesin kamu.

## Tools

### `web_search`

Mencari informasi di DuckDuckGo.

Input:

```json
{
  "query": "rust mcp server",
  "max_results": 10,
  "time_range": "week",
  "region": "id-id"
}
```

Parameter:

| Nama          | Wajib | Default | Keterangan                                                |
| ------------- | ----- | ------- | --------------------------------------------------------- |
| `query`       | Ya    | -       | Kata kunci pencarian                                      |
| `max_results` | Tidak | `15`    | Jumlah maksimal hasil                                     |
| `time_range`  | Tidak | -       | `day`, `week`, `month`, atau `year`                       |
| `region`      | Tidak | `id-id` | Kode wilayah DuckDuckGo, contoh `id-id`, `us-en`, `wt-wt` |

### `fetch_web_content`

Mengambil halaman web publik dan mengembalikan Markdown.

Input:

```json
{
  "url": "https://example.com/article",
  "query": "bagian yang relevan"
}
```

Parameter:

| Nama    | Wajib | Default | Keterangan                                                          |
| ------- | ----- | ------- | ------------------------------------------------------------------- |
| `url`   | Ya    | -       | URL lengkap `http` atau `https`                                     |
| `query` | Tidak | -       | Jika diisi, konten difilter secara semantik memakai embedding lokal |

## Embedding Opsional

Jika `fetch_web_content` diberi `query`, server mencoba mengambil embedding dari:

```text
http://127.0.0.1:8081
```

Override dengan:

```bash
export LLAMA_EMBED_URL="http://127.0.0.1:8081"
```

Server mencoba endpoint native llama.cpp:

```text
/embedding
```

Lalu fallback ke endpoint kompatibel OpenAI:

```text
/v1/embeddings
```

Kalau embedding server offline, tool tetap mengembalikan Markdown biasa.

## Development

```bash
cargo check
cargo build
cargo run
```

Server membaca JSON-RPC per baris dari stdin dan menulis response JSON-RPC ke stdout.

## Catatan

- DuckDuckGo bisa memicu CAPTCHA. Jika terjadi, server mencoba membuka browser ke DuckDuckGo HTML.
- `fetch_web_content` memblokir localhost, IP private, link-local, broadcast, unspecified, dan range CGNAT.
- Halaman dengan Cloudflare atau bot challenge biasanya tidak bisa dibaca langsung.
