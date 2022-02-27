use std::{path::PathBuf, net::IpAddr};
use std::str::FromStr;
use regex::Regex;

#[derive(Debug)]
pub enum DnsmasqParsedLine {
    Start { version: String, cache_size: u32 },
    NameServer(String),
    ReadHosts { path: PathBuf, address_count: u32 },
    Query { id: u64, source: String, query: String, domain: String, from: IpAddr },
    // Forwarded { id: u64, to: IpAddr },
    Reply { id: u64, cached: bool, domain: String, result: Option<IpAddr> }
}

pub struct DnsmasqLineParser {
    // parsing regexes
    regex_version_info: Regex,
    regex_nameserver: Regex,
    regex_read_hosts: Regex,
    regex_query: Regex,
    regex_reply: Regex,
    regex_reply_cached: Regex,
    
}

impl DnsmasqLineParser {

    pub fn new() -> Result<Self, regex::Error>  {
        Ok(DnsmasqLineParser {
            regex_version_info: Regex::new(r"dnsmasq\[\d+\]: started, version (?P<version>(?:\d+\.?)+) cachesize (?P<cachesize>\d+)")?,
            regex_nameserver: Regex::new(r"dnsmasq\[\d+\]: using nameserver (?P<address>(?:[\d\.])+(?:\#\d+)?)")?,
            regex_read_hosts: Regex::new(r"dnsmasq\[\d+\]: read (?P<path>(/\S+)+) - (?P<address_count>\d+) addresses")?,
            regex_query: Regex::new(r"dnsmasq\[\d+\]: (?P<id>\d+) (?P<source>\S+) query\[(?P<query>\w+)\] (?P<domain>[\w\.]+) from (?P<from>\S+)")?,
            regex_reply: Regex::new(r"dnsmasq\[\d+\]: (?P<id>\d+) (?P<source>\S+) reply (?P<domain>[\w\.]+) is (?P<ip>\S+)")?,
            regex_reply_cached: Regex::new(r"dnsmasq\[\d+\]: (?P<id>\d+) (?P<source>\S+) cached (?P<domain>[\w\.]+) is (?P<ip>\S+)")?,
        })
    }

    pub fn parse_line(&self, line: impl AsRef<str>) -> Option<DnsmasqParsedLine> {
        // most common lines
        
        if let Some(captures) = self.regex_query.captures(line.as_ref()) {
            let id: u64 = captures.name("id")?.as_str().parse().ok()?;
            let source: String = captures.name("source")?.as_str().to_string();
            let query: String = captures.name("query")?.as_str().to_string();
            let domain: String = captures.name("domain")?.as_str().to_string();
            let from: IpAddr = IpAddr::from_str(captures.name("from")?.as_str()).ok()?;

            return Some(DnsmasqParsedLine::Query {
                id,
                source,
                query,
                domain,
                from
            });
        }

        if let Some(captures) = self.regex_reply_cached.captures(line.as_ref()) {
            let id: u64 = captures.name("id")?.as_str().parse().ok()?;
            let domain: String = captures.name("domain")?.as_str().to_string();
            let ip: IpAddr = IpAddr::from_str(captures.name("ip")?.as_str()).ok()?;

            return Some(DnsmasqParsedLine::Reply {
                id,
                cached: true,
                domain,
                result: Some(ip)
            });
        }
        
        if let Some(captures) = self.regex_reply.captures(line.as_ref()) {
            let id: u64 = captures.name("id")?.as_str().parse().ok()?;
            let domain: String = captures.name("domain")?.as_str().to_string();
            let ip: Option<IpAddr> = IpAddr::from_str(captures.name("ip")?.as_str()).ok();

            return Some(DnsmasqParsedLine::Reply {
                id,
                cached: false,
                domain,
                result: ip
            });
        }



        // ---


        if let Some(captures) = self.regex_version_info.captures(line.as_ref()) {
            let version: String = captures.name("version")?.as_str().to_string();
            let cache_size: u32 = captures.name("cachesize")?.as_str().parse().ok()?;

            return Some(DnsmasqParsedLine::Start { version, cache_size });
        }

        if let Some(captures) = self.regex_read_hosts.captures(line.as_ref()) {
            let path: PathBuf = PathBuf::from(captures.name("path")?.as_str());
            let address_count: u32 = captures.name("address_count")?.as_str().parse().ok()?;

            return Some(DnsmasqParsedLine::ReadHosts { path, address_count });
        }

        if let Some(captures) = self.regex_nameserver.captures(line.as_ref()) {
            let server: String = captures.name("address")?.as_str().to_string();

            return Some(DnsmasqParsedLine::NameServer(server));
        }


        None
    }
}

#[cfg(test)]
mod tests {
    use std::{net::IpAddr, str::FromStr};

    use super::{DnsmasqLineParser, DnsmasqParsedLine};

    #[test]
    fn line_parser() {
        let parser = DnsmasqLineParser::new().unwrap();
        let parsed = parser.parse_line("dnsmasq[23894]: started, version 2.80 cachesize 150").unwrap();
        if let DnsmasqParsedLine::Start { version, cache_size} = parsed {
            assert_eq!(version, "2.80");
            assert_eq!(cache_size, 150);
        } else {
            panic!("parsed line had wrong type: {:?}", parsed);
        }

        let parsed = parser.parse_line("dnsmasq[25921]: using nameserver 172.17.0.1#53").unwrap();
        if let DnsmasqParsedLine::NameServer(server_addr) = parsed {
            assert_eq!(server_addr, "172.17.0.1#53");
        } else {
            panic!("parsed line had wrong type: {:?}", parsed);
        }


        let parsed = parser.parse_line("dnsmasq[525]: 1 127.0.0.1/42332 query[A] www.matthiaskind.com from 127.0.0.1").unwrap();
        if let DnsmasqParsedLine::Query{ id, domain, from, query, source } = parsed {
            assert_eq!(id, 1);
            assert_eq!(domain, "www.matthiaskind.com");
            assert_eq!(source, "127.0.0.1/42332");
            assert_eq!(query, "A");
            assert_eq!(from, IpAddr::from_str("127.0.0.1").unwrap());
        } else {
            panic!("parsed line had wrong type: {:?}", parsed);
        }

    }
}
