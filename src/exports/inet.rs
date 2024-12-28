use std::{net::{IpAddr, AddrParseError}, str::FromStr};

use ipnet::IpNet;

enum UserNetAddr {
    Address(IpAddr),
    Network(IpNet),
}
impl UserNetAddr {
    fn within(&self, net: IpNet) -> bool {
        match self {
            UserNetAddr::Address(addr) => net.trunc().contains(addr),
            UserNetAddr::Network(netw) => net.trunc().contains(netw),
        }
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

/// IP_CONTAINS(ip, subnet, [NULL|mask|mask_length]) -> NULL|bool
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

// SUPERNET() = aggregate function, returns common subnet address+length of all IP addresses provided into it

// IP_ADDRINDEX(number, subnet[, mask\][, NULL|'null'|'wrap'|'saturate'\]) = Nth address in subnet. 0 = truncated, 1 = first address, -1 = last/broadcast address, -2 = second last, ...
// third argument is wrapping strategy for out-of-bounds requests

// IP_ASINT(address) = to integer, primarily for sorting purposes

// DNS functions?
// IP reverse lookup / DNS lookup
