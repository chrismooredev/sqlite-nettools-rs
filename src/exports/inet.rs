use std::{net::{IpAddr, AddrParseError, Ipv4Addr, Ipv6Addr}, str::FromStr, fmt};

use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use rusqlite::types::ValueRef;

#[derive(thiserror::Error, Debug)]
pub enum InetError {
    #[error("Attempted to convert blob into IP Address/Network that has bad size {} (blob contents: {:x?}). Blobs of size 4,5,16,17 are expected (v4/v6 address bytes, optional prefix length)", Vec::len(.0), if .0.len() < 20 { .0.as_slice() } else { &.0[..20] })]
    UnrecognizedBlobLength(Vec<u8>),
    #[error("Attempt to use an invalid network mask")]
    InvalidNetworkMask(UserNetAddr, String),
    #[error("Found multiple network mask lenghts for one address. Address field provided {0}, but recieved additional mask {1:?} in argument {2}")]
    MultipleNetworkMasks(UserNetAddr, usize, String),
}

#[derive(Debug, Clone, Copy)]
pub enum UserNetAddr {
    Address(IpAddr),
    Network(IpNet),
}
impl UserNetAddr {
    pub fn address(&self) -> IpAddr {
        match self {
            UserNetAddr::Address(addr) => *addr,
            UserNetAddr::Network(net) => net.addr()
        }
    }
    pub fn within(&self, net: IpNet) -> bool {
        match self {
            UserNetAddr::Address(addr) => net.trunc().contains(addr),
            UserNetAddr::Network(netw) => net.trunc().contains(netw),
        }
    }
    fn from_ctx(ctx: &rusqlite::functions::Context<'_>, net: usize, mask: Option<usize>) -> rusqlite::Result<Option<UserNetAddr>> {
        if ctx.len() <= net { return Ok(None); }

        // pull a blob or string based address or network out of 'net'
        let netraw = ctx.get_raw(net);
        let mut una: UserNetAddr = match netraw {
            ValueRef::Null => return Ok(None),
            ValueRef::Blob(dat) if dat.len() == 4 => { // IPv4
                let raw: [u8; 4] = dat.try_into().unwrap();
                UserNetAddr::Address(IpAddr::from(raw))
            },
            ValueRef::Blob(dat) if dat.len() == 5 => { // IPv4/CIDR
                let raw: [u8; 4] = dat[..4].try_into().unwrap();
                let len = dat[4];

                let network = Ipv4Net::new(Ipv4Addr::from(raw), len)
                    .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

                UserNetAddr::Network(IpNet::V4(network))
            },
            ValueRef::Blob(dat) if dat.len() == 16 => { // IPv6
                let raw: [u8; 16] = dat.try_into().unwrap();
                UserNetAddr::Address(IpAddr::from(raw))
            },
            ValueRef::Blob(dat) if dat.len() == 16 => { // IPv6/CIDR
                let raw: [u8; 16] = dat[..4].try_into().unwrap();
                let len = dat[16];

                let network = Ipv6Net::new(Ipv6Addr::from(raw), len)
                    .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

                UserNetAddr::Network(IpNet::V6(network))
            },
            ValueRef::Blob(b) => return Err(rusqlite::Error::UserFunctionError(Box::new(InetError::UnrecognizedBlobLength(b.to_vec())))),
            ValueRef::Real(_) | ValueRef::Integer(_) => {
                // don't support turning integers or floats into addresses or networks
                let _s: String = ctx.get(net)?;
                unreachable!("pre-validated that this type is not a string")
            },
            ValueRef::Text(_) => {
                // delegate to existing from_str impl
                UserNetAddr::from_str(netraw.as_str().unwrap())
                    .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?
            }
        };

        // check the mask index for an appropriate mask
        // add it into una if possible
        if let Some(mask_idx) = mask {
            if ctx.len() > mask_idx {
                // try to pull something useful out of the mask argument
                // only take integer prefix, string integer, string mask

                let max_len = match una {
                    UserNetAddr::Address(IpAddr::V4(_)) => 32,
                    UserNetAddr::Address(IpAddr::V6(_)) => 128,
                    UserNetAddr::Network(IpNet::V4(_)) => 32,
                    UserNetAddr::Network(IpNet::V6(_)) => 128,
                };

                let len: Option<u8> = match ctx.get_raw(mask_idx) {
                    ValueRef::Null => None,
                    ValueRef::Integer(i) => {
                        if 0 <= i && i <= max_len {
                            // we are within proper range as an integer
                            Some(i as u8)
                        } else {
                            return Err(rusqlite::Error::UserFunctionError(Box::new(ipnet::PrefixLenError)));
                        }
                    },
                    ValueRef::Text(_) => {
                        let s = ctx.get_raw(mask_idx).as_str()?;

                        // IP mask or stringified integer
                        match u8::from_str(s) {
                            Ok(n) => {
                                if n <= max_len as u8 {
                                    Some(n)
                                } else {
                                    return Err(rusqlite::Error::UserFunctionError(Box::new(ipnet::PrefixLenError)));
                                }
                            },
                            Err(_pie) => {
                                match Ipv4Addr::from_str(s) {
                                    Ok(mask) => {
                                        Some(ipnet::ipv4_mask_to_prefix(mask)
                                            .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?)
                                    },
                                    Err(_) => {
                                        // IPv6 network masks aren't a thing - so error out
                                        return Err(rusqlite::Error::UserFunctionError(Box::new(InetError::InvalidNetworkMask(una, s.to_owned()))))
                                    }
                                }
                            },
                        }
                    },
                    ValueRef::Real(_) | ValueRef::Blob(_) => {
                        // manually trigger a bad type error
                        let _: String = ctx.get(mask_idx)?;
                        unreachable!()
                    },
                };

                if let Some(prefixlen) = len {
                    match una {
                        UserNetAddr::Address(IpAddr::V4(addr)) => {
                            una = UserNetAddr::Network(IpNet::V4(Ipv4Net::new(addr, prefixlen).expect("prefix length was pre-validated")));
                        },
                        UserNetAddr::Address(IpAddr::V6(addr)) => {
                            una = UserNetAddr::Network(IpNet::V6(Ipv6Net::new(addr, prefixlen).expect("prefix length was pre-validated")));
                        },
                        UserNetAddr::Network(_) => {
                            return Err(rusqlite::Error::UserFunctionError(Box::new(InetError::MultipleNetworkMasks(una, mask_idx, format!("{:?}", ctx.get_raw(mask_idx))))));
                        }
                    }
                }
            }
        }

        Ok(Some(una))
    }
}
impl FromStr for UserNetAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let as_addr = IpAddr::from_str(s);
        match as_addr {
            Ok(ipaddr) => Ok(UserNetAddr::Address(ipaddr)),
            Err(e) => {
                let as_netw = IpNet::from_str(s);
                if let Ok(netw) = as_netw {
                    Ok(UserNetAddr::Network(netw))
                } else { // IpNet's parse error isn't helpful so always use the IpAddr parse error
                    Err(e)
                }
            }
        }
    }
}

impl fmt::Display for UserNetAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserNetAddr::Address(IpAddr::V4(addr)) => fmt::Display::fmt(addr, f),
            UserNetAddr::Address(IpAddr::V6(addr)) => fmt::Display::fmt(addr, f),
            UserNetAddr::Network(IpNet::V4(addr)) => fmt::Display::fmt(addr, f),
            UserNetAddr::Network(IpNet::V6(addr)) => fmt::Display::fmt(addr, f),
        }
    }
}

/// Receives a subnet mask from the context object provided. The subnet value must always be provided (null is allowed), the mask index must be provided but it's value is optional.
///
/// This function short-circuits: if CIDR notation is found in the subn_idx, then mask_idx will not be observed.
fn normalize_mask(ctx: &rusqlite::functions::Context<'_>, subn_idx: usize, mask_idx: usize) -> rusqlite::Result<Option<IpNet>> {
    let Some(subn) = ctx.get_raw(subn_idx).as_str_or_null()? else { return Ok(None); };

    let parse_err = match IpNet::from_str(subn) {
        Ok(subn) => return Ok(Some(subn)),
        Err(e) => e,
    };

    let mask = (ctx.len() >= mask_idx + 1).then(|| ctx.get_raw(mask_idx))
        .map(|v| v.as_str_or_null()).transpose()?.flatten();

    let subnet: IpNet = match mask {
        None => return Err(rusqlite::Error::UserFunctionError(Box::new(parse_err))),
        Some(mask) => {
            let network: IpAddr = subn.parse()
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            let mask: IpAddr = mask.parse()
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            let prefix_len = ipnet::ip_mask_to_prefix(mask)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            IpNet::new(network, prefix_len)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?
        }
    };

    Ok(Some(subnet))
}

/// IP_FORMAT(NULL|ip [,NULL|mask\] [,NULL|should_truncate\]) -> NULL|ip
///
/// Formats an IPv4/IPv6 address (with optional mask) to a normalized form.
///
/// If the last argument is TRUE, then the address will be truncated when a network mask is provided.
///
/// # Examples
/// |Call|Result|
/// |-|-|
/// |`IP_FORMAT('192.168.003.002')`|`'192.168.3.2'`|
/// |`IP_FORMAT('192.168.3.2/16')`|`'192.168.3.2/16'`|
/// |`IP_FORMAT('10.2.3.1', '255.255.255.0')`|`'10.2.3.1/24'`|
/// |`IP_FORMAT('10.2.3.1', '255.255.255.0', TRUE)`|`'10.2.3.0/24'`|
/// |`IP_FORMAT('fe80:0:0:0:2:03:0:aabb/10')`|`'fe80::2:3:0:aabb/10'`|
/// |`IP_FORMAT('fe80:0:0:0:2:03:0:aabb/10', TRUE)`|`'fe80::/10'`|
pub fn format(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<String>> {
    match normalize_mask(ctx, 0, 1) {
        // it was successfully parsed as a subnet mask
        Ok(Some(mut net)) => Ok(Some({
            let should_truncate: Option<bool> = ctx.get(ctx.len()-1).ok().flatten();
            if should_truncate.unwrap_or(false) {
                net = net.trunc();
            }
            net.to_string()
        })),
        _ => {
            // it should be an address, or something is misaligned
            let Some(addrstr) = ctx.get_raw(0).as_str_or_null()? else { return Ok(None); };
            let addr = IpAddr::from_str(addrstr)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            Ok(Some(addr.to_string()))
        }
    }
}

/// IP_CONTAINS(ip_or_network, subnet, [NULL|mask|mask_length]) -> NULL|bool
///
/// Tests if the IPv4/IPv6 address or network in the first argument, is contained in the subnet specified by the latter arguments.
///
/// It is an error to provide a subnet in **both** CIDR and subnet mask notation - one or the other must be used for the containing network.
///
/// When comparing two networks, the smaller one should be provided first, in CIDR form.
///
/// # Examples
/// |Call|Result|
/// |-|-|
/// |`IP_CONTAINS('128.231.61.3', '128.231.60.0/22')`|`TRUE`|
/// |`IP_CONTAINS('128.231.59.7', '128.231.60.0', '255.255.252.0')`|`FALSE`|
/// |`IP_CONTAINS('128.231.59.7', '128.231.60.0', 22)`|`FALSE`|
/// |`IP_CONTAINS('fe80::82fe:a2', 'fe80::/10')`|`TRUE`|
pub fn contains(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<bool>> {
    let Some(subject_str) = ctx.get_raw(0).as_str_or_null()? else { return Ok(None); };

    let subject: UserNetAddr = subject_str.parse()
        .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

    let Some(network) = normalize_mask(ctx, 1, 2)? else { return Ok(None); };

    Ok(Some(subject.within(network)))
}

/// Converts an IP address or Address portion of a CIDR subnet, into a binary blob.
///
/// This has two primary uses:
/// * Sorting addresses squentially
/// * Storing addresses compactly
///
/// # Examples
/// |Call|Result|
/// |-|-|
/// |`IP_BLOBIFY('127.0.0.1')`|...|
pub fn blobify(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<Vec<u8>>> {
    let Some(subject_str) = ctx.get_raw(0).as_str_or_null()? else { return Ok(None); };
    let subject: UserNetAddr = subject_str.parse()
        .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

    Ok(Some(match subject {
        UserNetAddr::Address(a) => match a {
            IpAddr::V4(ipv4) => ipv4.octets().to_vec(),
            IpAddr::V6(ipv6) => ipv6.octets().to_vec(),
        },
        UserNetAddr::Network(n) => match n {
            IpNet::V4(netv4) => {
                let mut v = netv4.addr().octets().to_vec();
                v.push(netv4.prefix_len());
                v
            },
            IpNet::V6(netv6) => {
                let mut v = netv6.addr().octets().to_vec();
                v.push(netv6.prefix_len());
                v
            }
        }
    }))
}

// pub fn split(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<String>> {

// }

// SUPERNET() = aggregate function, returns common subnet address+length of all IP addresses provided into it

// IP_ADDRINDEX(number, subnet[, mask\][, NULL|'null'|'wrap'|'saturate'\]) = Nth address in subnet. 0 = truncated, 1 = first address, -1 = last/broadcast address, -2 = second last, ...
// third argument is wrapping strategy for out-of-bounds requests

// IP_ASINT(address) = to integer, primarily for sorting purposes

// DNS functions?
// IP reverse lookup / DNS lookup
