# kurl-cli

`kurl` is a command-line tool inspired by `curl`, written in Rust. It aims to be compatible with `curl`'s most common flags while providing enhanced, easy-to-read debugging information.

## Features

- GET, POST, HEAD requests
- Custom headers (`-H`)
- POST data (`-d`)
- Include response headers in output (`-i`)
- Fetch headers only (`-I`)
- Verbose logging for debugging (`-v`, `-v -v`, `-v -v -v`)

## Installation

```bash
cargo install --path .
```

## Usage

```
kurl [FLAGS] [OPTIONS] <URL>
```

### Examples

**Simple GET request:**

```bash
kurl https://httpbin.org/get
```

**Include response headers:**

```bash
kurl -i https://httpbin.org/get
```

**Fetch headers only (HEAD request):**

```bash
kurl -I https://httpbin.org/get
```

**POST request with data:**

```bash
kurl -X POST -d "name=kurl&lang=rust" https://httpbin.org/post
```

**Send custom headers:**

```bash
kurl -H "X-Custom: Hello" https://httpbin.org/headers
```

**Verbose output for debugging:**

Use `-v` for info, `-v -v` for debug, and `-v -v -v` for trace-level output, which includes detailed request and response headers similar to `curl -v`.

```bash
# Level 1: Info
kurl -v https://tiye.me

# Level 2: Debug
kurl -v -v https://tiye.me

# Level 3: Trace (with > < header details)
kurl -v -v -v https://tiye.me
```

## Design

The core logic is built with:

- **Argument Parsing**: `argh`
- **HTTP Client**: `reqwest`
- **Logging**: `log` + `env_logger`

## License

MIT. Code mostly rendered by Gemini.
