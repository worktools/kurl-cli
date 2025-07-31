# kurl-cli

`kurl` is a command-line tool inspired by `curl`, written in Rust. It aims to be compatible with `curl`'s most common flags while providing enhanced, easy-to-read debugging information, especially for redirect chains.

## Features

- GET, POST, HEAD requests
- Custom headers (`-H`)
- Cookies (`-b`, `--cookie`)
- POST data (`-d`)
- Raw POST data (`--data-raw`)
- Response headers included in output by default
- Fetch headers only (`-I`)
- Follow redirects (`-L`), with each step of the redirect chain clearly separated.
- Insecure connections (`-k`)
- Save output to file (`-o`)
- Manual DNS resolution (`--resolve`)
- Connection timeout (`--connect-timeout`)
- Verbose logging for deep debugging (`-v`)

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

**Follow a redirect and show the full chain:**

When following redirects with `-L`, `kurl` will print the full response for each request, separated by a clear divider. This is excellent for debugging redirect issues.

```bash
kurl -L http://google.com
```
```
HTTP/1.1 301 Moved Permanently
location: http://www.google.com/
...

----------------------------------------
HTTP/1.1 200 OK
...

<!doctype html>...
```

**Save output to a file:**
(Headers are printed to the console, body is saved to the file)
```bash
kurl -o google.html https://google.com
```

**POST request with data:**

`kurl` automatically uses the `POST` method when `-d` or `--data-raw` is present.

```bash
kurl -d "name=kurl&lang=rust" https://httpbin.org/post
```

**POST request with raw JSON data:**

Unlike `-d`, `--data-raw` sends the data exactly as is. No extra processing is done, and no `Content-Type` header is automatically added.

```bash
kurl --data-raw '{"json": true}' -H "Content-Type: application/json" https://httpbin.org/post
```

**Send custom headers:**

```bash
kurl -H "X-Custom: Hello" https://httpbin.org/headers
```

**Send cookies:**

```bash
kurl -b "name=value; name2=value2" https://httpbin.org/cookies
```

**Allow insecure connections (e.g., for self-signed certificates):**
```bash
kurl -k https://self-signed.badssl.com/
```

**Verbose output for debugging:**

Use a single `-v` flag to enable the most detailed logging level. This is equivalent to `curl -v` and will show request/response headers and underlying network connection details (TCP, TLS).

```bash
kurl -v -L https://google.com
```

## Design

The core logic is built with:

- **Argument Parsing**: `argh`
- **HTTP Client**: `reqwest`
- **Logging**: `log` + `env_logger`

## License

MIT. Code mostly rendered by Gemini.