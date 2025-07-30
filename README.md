# kurl-cli

`kurl` is a command-line tool inspired by `curl`, written in Rust. It aims to be compatible with `curl`'s most common flags while providing enhanced, easy-to-read debugging information.

## Features

- GET, POST, HEAD requests
- Custom headers (`-H`)
- POST data (`-d`)
- Response headers included in output by default
- Fetch headers only (`-I`)
- Follow redirects (`-L`)
- Insecure connections (`-k`)
- Save output to file (`-o`)
- Manual DNS resolution (`--resolve`)
- Connection timeout (`--connect-timeout`)
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

**Simple GET request (headers are included by default):**

```bash
kurl https://httpbin.org/get
```

**Fetch headers only (HEAD request):**

```bash
kurl -I https://httpbin.org/get
```

**Follow a redirect:**

```bash
kurl -L http://google.com
```

**Save output to a file:**
(Headers are printed to the console, body is saved to the file)
```bash
kurl -o google.html https://google.com
```

**POST request with data:**

```bash
kurl -X POST -d "name=kurl&lang=rust" https://httpbin.org/post
```

**Send custom headers:**

```bash
kurl -H "X-Custom: Hello" https://httpbin.org/headers
```

**Allow insecure connections (e.g., for self-signed certificates):**
```bash
kurl -k https://self-signed.badssl.com/
```

**Verbose output for debugging:**

Use `-v` for info (includes timing), `-v -v` for debug, and `-v -v -v` for trace-level output.

```bash
# Level 1: Info with timing
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
