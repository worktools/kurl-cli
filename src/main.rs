use argh::FromArgs;
use log::{debug, error, info, LevelFilter};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::str::FromStr;
use std::time::{Duration, Instant};

#[derive(FromArgs, Debug)]
/// A simple curl clone, written in Rust.
struct Cli {
    /// the URL to request
    #[argh(positional)]
    url: String,

    /// fetch the headers only (HTTP HEAD)
    #[argh(switch, short = 'I')]
    head: bool,

    /// the HTTP method to use
    #[argh(option, short = 'X', default = "\"GET\".to_string()") ]
    request: String,

    /// the data to send in a POST request
    #[argh(option, short = 'd')]
    data: Option<String>,

    /// custom header(s) to pass to the server
    #[argh(option, short = 'H')]
    headers: Vec<String>,

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

    /// increase logging verbosity
    #[argh(switch, short = 'v')]
    verbose: u8,
}

fn get_log_level(verbose_count: u8) -> LevelFilter {
    match verbose_count {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
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

fn format_reqwest_error(e: &reqwest::Error) -> String {
    let url_str = e
        .url()
        .map_or_else(|| "the requested URL".to_string(), |url| url.to_string());

    let mut msg = String::new();

    if e.is_timeout() {
        msg.push_str(&format!("Connection to {} timed out.\n", url_str));
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
        msg.push_str(&format!("Failed to connect to {}.\n", url_str));
        msg.push_str("\nSuggestions:\n");
        msg.push_str("- Ensure the domain name is correct and the server is running.\n");
        msg.push_str("- A firewall, proxy, or network restrictions might be blocking the connection.\n");
        if e.to_string().contains("certificate") {
            msg.push_str("- The server's SSL certificate appears to be invalid. You can use the -k/--insecure flag to bypass this check (at your own risk).\n");
        }
    } else if e.is_redirect() {
        msg.push_str(&format!("Too many redirects for {}.\n", url_str));
        msg.push_str("\nSuggestions:\n");
        msg.push_str("- The server may have a misconfigured redirect loop.\n");
        msg.push_str("- Use `-v -v -v` to trace the redirect path.\n");
    } else if e.is_builder() {
        msg.push_str(&format!(
            "Internal error: Failed to build the HTTP request for {}.\n",
            url_str
        ));
        msg.push_str(&format!("Details: {}", e));
    } else {
        // Fallback for other error types (body, decode, etc.)
        msg.push_str(&format!(
            "An error occurred while processing the request to {}.\n",
            url_str
        ));
        msg.push_str(&format!("\nDetails: {}", e));
    }

    msg
}

fn main() {
    let cli: Cli = argh::from_env();
    let log_level = get_log_level(cli.verbose);

    env_logger::Builder::new()
        .filter_level(log_level)
        .init();

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

        let mut client_builder = Client::builder()
            .default_headers(headers.clone())
            .redirect(if cli.location {
                Policy::default()
            } else {
                Policy::none()
            })
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

        let mut method = cli.request.to_uppercase();
        if cli.head {
            method = "HEAD".to_string();
        }

        let request_builder = match method.as_str() {
            "HEAD" => client.head(&cli.url),
            "GET" => client.get(&cli.url),
            "POST" => {
                let mut req = client.post(&cli.url);
                if let Some(data) = cli.data.clone() {
                    if !headers.contains_key("content-type") {
                        req = req.header("Content-Type", "application/x-www-form-urlencoded");
                    }
                    req = req.body(data);
                }
                req
            }
            _ => client.request(method.parse()?, &cli.url),
        };

        let is_trace = log_level == LevelFilter::Trace;

        if is_trace {
            print_request(&request_builder);
        }

        let mut response: Response = request_builder.send()?;

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

        let mut body_bytes = Vec::new();
        if !cli.head {
            let status = response.status();
            response.read_to_end(&mut body_bytes)?;
            if !status.is_success() {
                error!("Request failed with status: {status}");
            }
        }

        if let Some(output_file) = cli.output {
            let mut file = File::create(&output_file)?;
            std::io::stdout().write_all(&header_output)?;
            file.write_all(&body_bytes)?;
            info!("Body written to {output_file}");
        } else {
            let mut stdout = std::io::stdout();
            stdout.write_all(&header_output)?;
            stdout.write_all(&body_bytes)?;
            stdout.flush()?;
        }

        if cli.verbose > 0 {
            info!("Request completed in {:?}", start_time.elapsed());
        }

        Ok(())
    };

    if let Err(e) = run() {
        if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
            eprintln!("kurl: error: {}", format_reqwest_error(reqwest_err));
        } else {
            eprintln!("kurl: error: {}", e);
        }
        std::process::exit(1);
    }
}

