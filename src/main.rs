use argh::FromArgs;
use log::{debug, error, LevelFilter};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::io::{Read, Write};
use std::str::FromStr;

#[derive(FromArgs, Debug)]
/// A simple curl clone, written in Rust.
struct Cli {
    /// the URL to request
    #[argh(positional)]
    url: String,

    /// include protocol response headers in the output
    #[argh(switch, short = 'i')]
    include: bool,

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
            eprintln!("> {} {} {:?}", built_req.method(), built_req.url().path(), built_req.version());
            eprintln!("> Host: {}", built_req.url().host_str().unwrap_or(""));
            for (name, value) in built_req.headers() {
                eprintln!("> {}: {}", name, value.to_str().unwrap_or("[non-ascii]"));
            }
            eprintln!(">");
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli: Cli = argh::from_env();
    let log_level = get_log_level(cli.verbose);

    env_logger::Builder::new()
        .filter_level(log_level)
        .init();

    debug!("Parsed arguments: {:?}", cli);

    let mut headers = HeaderMap::new();
    for header_str in &cli.headers {
        let parts: Vec<&str> = header_str.splitn(2, ':').collect();
        if parts.len() == 2 {
            let name = HeaderName::from_str(parts[0].trim())?;
            let value = HeaderValue::from_str(parts[1].trim())?;
            headers.insert(name, value);
        } else {
            return Err(format!("Invalid header format: {}", header_str).into());
        }
    }

    let client = Client::builder().default_headers(headers.clone()).build()?;
    
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

    let mut response = request_builder.send()?;
    
    if is_trace {
        eprintln!("< {:?} {}", response.version(), response.status());
        for (key, value) in response.headers() {
            eprintln!("< {}: {}", key, value.to_str().unwrap_or("[non-ascii]"));
        }
        eprintln!("<");
    }

    let mut stdout = std::io::stdout();

    if (cli.head || cli.include) && !is_trace {
        writeln!(stdout, "{:?} {}", response.version(), response.status())?;
        for (key, value) in response.headers() {
            writeln!(stdout, "{}: {}", key, value.to_str()?)?;
        }
        writeln!(stdout)?;
    }

    if !cli.head {
        let status = response.status();
        let mut body_bytes = Vec::new();
        response.read_to_end(&mut body_bytes)?;
        
        if status.is_success() {
            stdout.write_all(&body_bytes)?;
        } else {
            error!("Request failed with status: {}", status);
            stdout.write_all(&body_bytes)?;
        }
    }
    
    stdout.flush()?;

    Ok(())
}
