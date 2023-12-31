# kenshi

A tool that focuses on medium-sized scans, same as [ffuf](https://github.com/ffuf/ffuf) but also different.
It is currently in its infancy

![demo](static/demo.png)

## The following has now been done

- [x] Performance testing
- [x] Bulk URL loading tests
- [x] socks5/http/https proxy support
- [x] Reduced DNS resolution failures
- [x] And...

## Experimental planning:

- [x] Try to use lua/python to implement modular customized scanning (e.g. sql injection detection, data extraction, sqlmap linkage...)
- [ ] Polling read unbounded wordlist
- [ ] dns preheat, avoiding dns record resolution failures
- [ ] ipv6
- [ ] mongodb...

**Suggestions are always welcome**

## Notes on usage

We need to care about the meaning of the following parameters

### Debug mode(default)

used to get detailed parameters of the target, to decide on optimization parameters, and to debug network errors (similar to ffuf scanning results).

### detail mode

get detailed scanning status, with progress bar and HTTP/IO related status display.

- Parameter -v or --stats

### Silent mode

only need results and speed, don't care about everything.

- Parameter --silent

# performance test

### Test Configuration

- timeout: 10s
- request retry: 1
- matching string: admin
- Match status code: 200
- follow redirects: disabled

### Environment configuration

- Fedora 37 kernel: 6.3.12-100 x86_64
- System memory 32g
- Bandwidth 1Gbps
- Processor 28 cores
- Configured sysctl.conf
- rustc 1.67.0 (fc594f156 2023-01-24)

### Test commands

```bash
time ffuf -u https://FUZZ/robots.txt -w 10m-domains_public.txt -timeout 10 -mr admin -t 5000 -of csv -o ffuf.csv
time httpx -l robots.txt_public_urls.txt -rl 99999 -t 5000 -mc 200 -retries 1 -timeout 10 -mr admin -o httpx.out
time wfuzz -t 30 --ss admin -w $PWD/public_domains-10000.txt -Z https://FUZZ/robots.txt # only threads
time kenshi -u https://FUZZ/robots.txt -w 10m-domains_public.txt -c 5000 --mr admin -v -o ken.out
```


#### 10m Wordlist (tested in: 2023/07/30)

| Project | Wordlist 10,000,000/lines | Version | Time Consumption | Hits | Maximum Memory Usage | Result |
| --- | --- | --- | --- | --- | --- | --- |
| [ffuf](https://github.com/ffuf/ffuf) | 10,000,000 | v2.0.0-dev | ？   | ?   | 22g+ | systemd-oomd killed |
| [httpx](https://github.com/projectdiscovery/httpx) | 10,000,000 | v1.3.4 | ？   | ?   | 10g++ | systemd-oomd killed |
| kenshi | 10,000,000 | v0.1.2 | 57m5.310s | 3219982 | 8835M | complete |


#### 100,000 Wordlist (tested in: 2023/07/30)

| Project | Wordlist 100,000/lines | Version | Time Consumption | Hits | Maximum Memory Usage | Result |
| --- | --- | --- | --- | --- | --- | --- |
| [ffuf](https://github.com/ffuf/ffuf) | 100,000 | v2.0.0-dev | 0m54.301s | 10950 | 1076M   | complete |
| [httpx](https://github.com/projectdiscovery/httpx) | 100,000 | v1.3.4 | 1m4.815s | 11995 | 1060M | complete |
| [wfuzz](https://github.com/xmendez/wfuzz) | 100,000 | 3.1.0 | 75m34.224s   | 12325   | 462M | complete |
| kenshi | 100,000 | v0.1.2 | 0m32.100s | 11980 | 2531M | complete |


# Install kenshi

```sh
# Cargo required.
cargo install --git https://github.com/AM8bit/kenshi
```

# Build

1.  Installing Rust

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

3.  Get source code

```sh
git clone https://github.com/AM8bit/kenshi.git
cd kenshi
make
./kenshi --help
```

# Usage

The definition of [ffuf](https://github.com/ffuf/ffuf) has been adopted for some of the parameters and descriptions

`kenshi -h`

```console
Usage: kenshi [options]

Options:
    -u, --url <url>     required. Test url
    -w, --wordlist <file>
                        required. Wordlist file path. eg. '/path/to/wordlist'
    -o, --output <file> Output result
        --or-match      Any one of these hits will do. (default: and)
        --mc <200,403,401,500>
                        Match HTTP status codes, or "all" for everything.
        --mr <regexp>   Match regexp
        --ms <int>      Match HTTP response size
        --ml <int>      Match amount of lines in response
        --or-filter     Any one of these hits will do. (default: and)
        --fc <int,...>  Filter HTTP status codes from response. Comma
                        separated list of codes and ranges
        --fl <int,...>  Filter by amount of lines in response. Comma separated
                        list of line counts and ranges. eg. --fl 123,1234
        --fr <regexp>   Filter regexp
        --fs <rules...> Filter HTTP response size. Comma separated list of
                        sizes and ranges. eg. --fs "<100,>1000,10-50,1234"
        --rt <int>      Request timeout seconds
    -c, --concurrent <int>
                        Number of concurrent requests. default: 500
        --follow-redirect <int>
                        enable redirect 301/302. disabled by default
    -r, --retries <int> Number of failed retry requests
    -x, --proxy <socks5://1.1.1.1:1080>
                        proxy request, http/https/socks5
    -U, --auth <username:password>
                        proxy auth, if required
    -D                  Replace wordlist %EXT% keywords with extension. Used
                        in conjunction with -e flag. (default: false)
    -e, --ext           Comma separated list of extensions. Extends FUZZ
                        keyword.
    -s, --script        lua script(This is an experimental feature)
        --silent        silent mode
    -v, --stats         Display detailed scanning status
        --vv            show version
    -h, --help          print this help menu

```

### examples

#### Basic Scan

- `kenshi -u https://example.com/FUZZ -w fuzz.dict`

#### Match page string

- `kenshi -u https://FUZZ/robots.txt -w fuzz.dict --mr "test_str[\d]{0,5}"`

#### Match status code

- `kenshi -u https://FUZZ/robots.txt -w fuzz.dict --mc 200,403`

#### Replace %EXT%, scanning for specified extensions (compatible with dirsearch -e)

- `kenshi -u https://example.com/FUZZ -w fuzz.dict -D -e php,json,conf --mc 200`

#### Maximum number of follow redirects

- `kenshi -u https://example.com/FUZZ -w fuzz.dict --mc 200 --follow-redirect 2`

#### Receive stdin wordlist

- `cat fuzz.dict | kenshi -u https://example.com/FUZZ`

#### Configuring the request proxy (offensive scanning)

- `kenshi -u https://example.com/FUZZ -D -e zip -w fuzz.dict -x http://12.12.12.12:1080 -U pwn:123123 -c 1000`

#### Exclude Scanning

- `kenshi -u https://example.com/FUZZ -w fuzz.dict --fl 10 --mc 200 --mr test_str`
- `kenshi -u https://example.com/FUZZ -w fuzz.dict --fc 403,404,500,400`

### Scanning results

- Currently only support text format, the structure is very simple, each line a hit url, no more information.

# License

kenshi is distributed under MIT License

