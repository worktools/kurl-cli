use argh::FromArgs;
use log::{debug, error, info};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::str::FromStr;
use std::time::{Duration, Instant};

#[derive(FromArgs, Debug)]
/// A curl clone with detailed debugging info, written in Rust.
struct Cli {
    /// the URL to request
    #[argh(positional)]
    url: String,

    /// fetch the headers only (HTTP HEAD)
    #[argh(switch, short = 'I')]
    head: bool,

    /// the HTTP method to use
    #[argh(option, short = 'X', default = "\"GET\".to_string()")]
    request: String,

    /// the data to send in a POST request
    #[argh(option, short = 'd')]
    data: Option<String>,

    /// raw data to send in a POST request, without processing
    #[argh(option)]
    data_raw: Option<String>,

    /// custom header(s) to pass to the server
    #[argh(option, short = 'H')]
    headers: Vec<String>,

    /// cookie(s) to pass to the server, e.g. "name=value; name2=value2"
    #[argh(option, short = 'b', long = "cookie")]
    cookie: Option<String>,

    /// follow redirects
    #[argh(switch, short = 'L')]
    location: bool,

    /// allow insecure server connections
    #[argh(switch, short = 'k')]
    insecure: bool,

    /// resolve a host to a specific IP address
    #[argh(option)]
    resolve: Vec<String>,

    /// maximum time in seconds that you allow the connection to the server to take
    #[argh(option)]
    connect_timeout: Option<u64>,

    /// write output to <file> instead of stdout
    #[argh(option, short = 'o')]
    output: Option<String>,

    /// enable verbose output, including request headers, response headers, and network-level logs.
    #[argh(switch, short = 'v')]
    verbose: bool,
}

fn print_request(req: &RequestBuilder) {
    if let Some(req_clone) = req.try_clone() {
        if let Ok(built_req) = req_clone.build() {
            eprintln!(
                "> {} {} {:?}",
                built_req.method(),
                built_req.url().path(),
                built_req.version()
            );
            eprintln!("> Host: {}", built_req.url().host_str().unwrap_or(""));
            for (name, value) in built_req.headers() {
                eprintln!("> {}: {}", name, value.to_str().unwrap_or("[non-ascii]"));
            }
            eprintln!(">");
        }
    }
}

fn normalize_url(url: &str) -> String {
    // check if the URL already has a protocol
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }

    // otherwise, prepend http://
    format!("http://{url}")
}

fn format_reqwest_error(e: &reqwest::Error) -> String {
    let url_str = e
        .url()
        .map_or_else(|| "the requested URL".to_string(), |url| url.to_string());

    let mut msg = String::new();

    if e.is_timeout() {
        msg.push_str(&format!("Connection to {url_str} timed out.\n"));
        msg.push_str("\nSuggestions:\n");
        msg.push_str("- Check your network connection and proxy settings.\n");
        msg.push_str(
            "- The server might be slow or overloaded. Try increasing the timeout with --connect-timeout.\n",
        );
        if url_str.contains("github.com") {
            msg.push_str(
                "- Accessing GitHub from some regions can be slow. Consider using a VPN or a proxy.\n",
            );
        }
    } else if e.is_connect() {
        msg.push_str(&format!("Failed to connect to {url_str}.\n"));
        msg.push_str("\nSuggestions:\n");
        msg.push_str("- Ensure the domain name is correct and the server is running.\n");
        msg.push_str(
            "- A firewall, proxy, or network restrictions might be blocking the connection.\n",
        );
        if e.to_string().contains("certificate") {
            msg.push_str("- The server's SSL certificate appears to be invalid. You can use the -k/--insecure flag to bypass this check (at your own risk).\n");
        }
    } else if e.is_redirect() {
        msg.push_str(&format!("Too many redirects for {url_str}.\n"));
        msg.push_str("\nSuggestions:\n");
        msg.push_str("- The server may have a misconfigured redirect loop.\n");
        msg.push_str("- Use `-v -v -v` to trace the redirect path.\n");
    } else if e.is_builder() {
        msg.push_str(&format!(
            "Internal error: Failed to build the HTTP request for {url_str}.\n"
        ));
        msg.push_str(&format!("Details: {e}"));
    } else {
        // Fallback for other error types (body, decode, etc.)
        msg.push_str(&format!(
            "An error occurred while processing the request to {url_str}.\n"
        ));
        msg.push_str(&format!("\nDetails: {e}"));
    }

    msg
}

fn main() {
    let cli: Cli = argh::from_env();
    let mut builder = env_logger::Builder::new();
    if cli.verbose {
        builder.parse_filters("kurl=trace,reqwest=trace,hyper=trace");
    } else {
        builder.parse_filters("kurl=warn");
    }
    builder.init();

    debug!("Parsed arguments: {cli:?}");

    let run = || -> Result<(), Box<dyn Error>> {
        let start_time = Instant::now();

        let mut headers = HeaderMap::new();
        for header_str in &cli.headers {
            let parts: Vec<&str> = header_str.splitn(2, ':').collect();
            if parts.len() == 2 {
                let name = HeaderName::from_str(parts[0].trim())?;
                let value = HeaderValue::from_str(parts[1].trim())?;
                headers.insert(name, value);
            } else {
                return Err(format!("Invalid header format: {header_str}").into());
            }
        }

        if let Some(cookie_str) = &cli.cookie {
            headers.insert(reqwest::header::COOKIE, HeaderValue::from_str(cookie_str)?);
        }

        let mut client_builder = Client::builder()
            .user_agent(concat!("kurl/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers.clone())
            .redirect(Policy::none())
            .danger_accept_invalid_certs(cli.insecure);

        for r in &cli.resolve {
            let parts: Vec<&str> = r.splitn(3, ':').collect();
            if parts.len() == 3 {
                let host = parts[0];
                let port = parts[1].parse::<u16>()?;
                let ip_addr = parts[2].parse::<std::net::IpAddr>()?;
                let socket_addr = std::net::SocketAddr::new(ip_addr, port);
                client_builder = client_builder.resolve(host, socket_addr);
            } else {
                return Err(
                    format!("Invalid resolve format: {r}. Expected <host>:<port>:<ip>").into(),
                );
            }
        }

        if let Some(timeout) = cli.connect_timeout {
            client_builder = client_builder.connect_timeout(Duration::from_secs(timeout));
        }

        let client = client_builder.build()?;

        let initial_method = if cli.head {
            "HEAD".to_string()
        } else if (cli.data.is_some() || cli.data_raw.is_some())
            && cli.request.to_uppercase() == "GET"
        {
            "POST".to_string()
        } else {
            cli.request.to_uppercase()
        };

        let is_trace = cli.verbose;
        let mut current_url = normalize_url(&cli.url);
        let mut redirect_count = 0;
        const MAX_REDIRECTS: u8 = 10;

        loop {
            if cli.data.is_some() && cli.data_raw.is_some() {
                return Err("Cannot use both --data and --data-raw at the same time".into());
            }

            let method = if redirect_count > 0 {
                "GET"
            } else {
                &initial_method
            };

            let request_builder = match method {
                "HEAD" => client.head(&current_url),
                "GET" => client.get(&current_url),
                "POST" => {
                    let mut req = client.post(&current_url);
                    if let Some(data) = cli.data.clone() {
                        if !headers.contains_key("content-type") {
                            req = req.header("Content-Type", "application/x-www-form-urlencoded");
                        }
                        req = req.body(data);
                    } else if let Some(data) = cli.data_raw.clone() {
                        req = req.body(data);
                    }
                    req
                }
                other => client.request(other.parse()?, &current_url),
            };

            if is_trace {
                print_request(&request_builder);
            }

            let mut response: Response = request_builder.send()?;
            let status = response.status();

            if is_trace {
                eprintln!("< {:?} {}", response.version(), response.status());
                for (key, value) in response.headers() {
                    eprintln!("< {}: {}", key, value.to_str().unwrap_or("[non-ascii]"));
                }
                eprintln!("<");
            }

            let mut header_output: Vec<u8> = Vec::new();
            if !is_trace {
                writeln!(
                    header_output,
                    "{:?} {}",
                    response.version(),
                    response.status()
                )?;
                for (key, value) in response.headers() {
                    writeln!(header_output, "{}: {}", key, value.to_str()?)?;
                }
                writeln!(header_output)?;
            }

            let next_url = if status.is_redirection() {
                response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|location| response.url().join(location).ok())
                    .map(|u| u.to_string())
            } else {
                None
            };

            let mut body_bytes = Vec::new();
            if !cli.head {
                response.read_to_end(&mut body_bytes)?;
                if !status.is_success() && !status.is_redirection() {
                    error!("Request failed with status: {status}");
                }
            }

            if let Some(output_file) = &cli.output {
                let mut file = File::create(output_file)?;
                std::io::stdout().write_all(&header_output)?;
                file.write_all(&body_bytes)?;
                info!("Body written to {output_file}");
            } else {
                let mut stdout = std::io::stdout();
                stdout.write_all(&header_output)?;
                stdout.write_all(&body_bytes)?;
                stdout.flush()?;
            }

            if cli.location && next_url.is_some() {
                if redirect_count >= MAX_REDIRECTS {
                    return Err("Too many redirects".into());
                }
                redirect_count += 1;
                current_url = next_url.unwrap();
                writeln!(
                    std::io::stdout(),
                    "\n----------------------------------------"
                )?;
                continue;
            }

            break;
        }

        if cli.verbose {
            info!("Request completed in {:?}", start_time.elapsed());
        }

        Ok(())
    };

    if let Err(e) = run() {
        if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
            eprintln!("kurl: error: {}", format_reqwest_error(reqwest_err));
        } else {
            eprintln!("kurl: error: {e}");
        }
        std::process::exit(1);
    }
}
