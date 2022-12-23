use std::{net::IpAddr, str::FromStr};

use ipnet::IpNet;

/// A collection of SQLite functions for dealing with MAC addresses, and their associated vendor affiliations (OUIs).
/// 
/// Each function accepts MAC addresses in varying formats (though only the first is shown in example usages for brevity)
/// * `aa-bb-cc-dd-ee-ff`
/// * `aa:bb:cc:dd:ee:ff`
/// * `aabb.ccdd.eeff`
/// * `aabbccddeeff`
/// * `0xaabbccddeeff`
/// 
/// See the [MAC_FORMAT](crate::exports::mac::format) function to convert MAC addresses between known formats.
pub mod mac {
    use crate::oui::{OuiMeta, Oui};

    #[derive(thiserror::Error, Debug)]
    enum MacFormatError {
        #[error("Mixed case format specifier is not allowed. Input case is used to determine output casing.")]
        MixedCaseFmtSpecifier,
        #[error("Bad format specifier provided (got {0:?}). Omit format specifier, or provide one of the following: (NULL, `hex`, `hexstring`), `hexadecimal`, `bare`, `dot`, `canonical`, `interface-id`, `link-local`)")]
        BadFmtSpecifier(String),
    }

    fn find_mac(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<(Oui, OuiMeta<&'static str>)>> {
        let Some(s) = ctx.get::<Option<String>>(0)? else { return Ok(None); };
        if s == "" { return Ok(None); }
        let mac = crate::parse_mac_addr(&s)
            .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

        Ok(crate::oui::EMBEDDED_DB.search_entry(mac))
    }

    /// # MAC_FORMAT(mac) -> mac'
    /// Formats a MAC address into a normalized form. Uses `hexstring` format by default.
    /// 
    /// The casing of the format string determines the casing of the output. Mixed-case output is not supported.
    /// 
    /// # Usage
    /// |Call|Result|
    /// |-|-|
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff')`                 | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', NULL)`           | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'hex')`          | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'HEX')`          | `'AA:BB:CC:DD:EE:FF'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'hexstring')`    | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'colon')`        | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'hexadecimal')`  | `'0xaabbccddeeff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'bare')`         | `'aabbccddeeff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'dot')`          | `'aabb.ccdd.eeff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'dash')`         | `'aa-bb-cc-dd-ee-ff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'canonical')`    | `'aa-bb-cc-dd-ee-ff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'interface-id')` | `'a8bb:ccff:fedd:eeff'` |
    /// |`MACFORMAT('aa-bb-cc-dd-ee-ff', 'link-local')`   | `'fe80::a8bb:ccff:fedd:eeff'` |
    pub fn format(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<String> {
        let mac_str: String = ctx.get(0)?;
        let mac = eui48::MacAddress::parse_str(&mac_str)
            .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

        let mut raw_fmt = (ctx.len() == 2).then(|| ctx.get::<Option<String>>(1)).transpose()?.flatten();
        let mut has_upper = false;
        if let Some(fmt) = raw_fmt.as_mut() {
            has_upper = fmt.contains(|c: char| c.is_ascii_uppercase());
            let has_lower = fmt.contains(|c: char| c.is_ascii_lowercase());
            if has_upper && has_lower {
                return Err(rusqlite::Error::UserFunctionError(Box::new(MacFormatError::MixedCaseFmtSpecifier)));
            }
            fmt.make_ascii_lowercase();
        }
        let mut formatted = match raw_fmt.as_ref().map(String::as_str) {
            None | Some("") | Some("hex") | Some("hexstring") | Some("colon") => mac.to_hex_string(),
            Some("hexadecimal") => mac.to_hexadecimal(),
            Some("bare") => mac.to_hexadecimal()[2..].to_string(),
            Some("dot") => mac.to_dot_string(),
            Some("canonical") | Some("dash") => mac.to_canonical(),
            Some("interface-id") => mac.to_interfaceid(),
            Some("link-local") => mac.to_link_local(),
            _ => return Err(rusqlite::Error::UserFunctionError(Box::new(MacFormatError::BadFmtSpecifier(raw_fmt.unwrap()))))
        };
        if has_upper { formatted.make_ascii_uppercase(); }
        Ok(formatted)
    }

    /// # MAC_PREFIX(NULL|mac) -> NULL|oui
    /// Returns the lowercase prefix for the provided MAC address.
    /// Returns either the first three bits, or CIDR style when the prefix is longer than 24 bits.
    /// (ex: `2b:ce:7a` or `5e:a5:c3:80:00:00/28`)
    /// 
    /// # Usage:
    /// |Call|Result|
    /// |-|-|
    /// |`MAC_PREFIX('3c-a6-f6-c4-34-f8')` | `'aa:bb:cc'`|
    /// |`MAC_PREFIX('8c-1c-da-82-4c-2e')` | `'8c:1c:da:80:00:00/28'`|
    /// |`MAC_PREFIX('33-33-00-00-00-01')` | `NULL`  |
    pub fn prefix(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<String>> {
        let mac = find_mac(ctx)?;
        Ok(mac.map(|(o, _om)| format!("{:?}", o)))
    }

    /// # MAC_MANUF(NULL|mac) -> NULL|manuf
    /// Returns the short manufacturer name belonging to this MAC's OUI
    /// 
    /// # Usage:
    /// |Call|Result|
    /// |-|-|
    /// |`MAC_MANUF('3c-a6-f6-c4-34-f8')` | `'Apple'`|
    /// |`MAC_MANUF('8c-1c-da-82-4c-2e')` | `'Atol'` |
    /// |`MAC_MANUF('33-33-00-00-00-01')` |  `NULL`  |
    pub fn manuf(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<&'static str>> {
        let mac = find_mac(ctx)?;
        Ok(mac.map(|(_o, om)| *om.manuf()))
    }

    /// # MAC_MANUFLONG(NULL|mac) -> NULL|manuf_long
    /// Returns the long manufacturer name belonging to this MAC's OUI
    /// 
    /// # Usage:
    /// |Call|Result|
    /// |-|-|
    /// |`MAC_MANUFLONG('3c-a6-f6-c4-34-f8')` | `'Apple, Inc.'`|
    /// |`MAC_MANUFLONG('8c-1c-da-82-4c-2e')` | `'Atol Llc'` |
    /// |`MAC_MANUFLONG('33-33-00-00-00-01')` |  `NULL`  |
    pub fn manuf_long(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<&'static str>> {
        let mac = find_mac(ctx)?;
        Ok(mac.and_then(|(_o, om)| om.manuf_long().copied()))
    }

    /// # MAC_COMMENT(NULL|mac) -> NULL|comment
    /// Returns the long manufacturer name belonging to this MAC's OUI
    /// 
    /// # Usage:
    /// |Call|Result|
    /// |-|-|
    /// |`MAC_COMMENT('3c-a6-f6-c4-34-f8')` | `NULL`|
    /// |`MAC_COMMENT('08-00-87-aa-bb-cc')` | `'terminal servers'`|
    /// |`MAC_COMMENT('33-33-00-00-00-01')` |  `NULL`  |
    pub fn comment(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<&'static str>> {
        let mac = find_mac(ctx)?;
        Ok(mac.and_then(|(_o, om)| om.comment().copied()))
    }

    macro_rules! gen_passthrough_body {
        ($fname: ident, $ctx: ident) => {{
            let raw_str: Option<String> = $ctx.get(0)?;
            let mac_str = match raw_str.as_deref() {
                None | Some("") => return Ok(None),
                Some(s) => s,
            };

            let mac = crate::parse_mac_addr(&mac_str)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            Ok(Some(mac.$fname()))
        }}
    }

    /// # MAC_ISUNICAST(NULL|mac) -> NULL|BOOL
    /// 
    /// Returns true if bit 1 of Y is 0 in address `xY:xx:xx:xx:xx:xx`
    pub fn is_unicast(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<bool>> {
        gen_passthrough_body!(is_unicast, ctx)
    }

    /// # MAC_ISMULTICAST(NULL|mac) -> NULL|BOOL
    /// 
    ///  Returns true if bit 1 of Y is 1 in address `xY:xx:xx:xx:xx:xx`
    pub fn is_multicast(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<bool>> {
        gen_passthrough_body!(is_multicast, ctx)
    }

    /// # MAC_ISUNIVERSAL(NULL|mac) -> NULL|BOOL
    /// 
    /// Returns true if bit 2 of Y is 0 in address `xY:xx:xx:xx:xx:xx`
    pub fn is_universal(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<bool>> {
        gen_passthrough_body!(is_universal, ctx)
    }

    /// # MAC_ISLOCAL(NULL|mac) -> NULL|BOOL
    /// 
    /// Returns true if bit 2 of Y is 1 in address `xY:xx:xx:xx:xx:xx`
    pub fn is_local(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<bool>> {
        gen_passthrough_body!(is_local, ctx)
    }
}



pub fn in_subnet(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<bool> {
    let args = ctx.len();
    // (ip, network, subnetmask) -> bool
    // (ip, cidr) -> bool
    let ipaddr_raw: String = ctx.get(0).unwrap();
    let ipaddr: IpAddr = ipaddr_raw.parse()
        .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

    let network = match args {
        2 => {
            let network_raw: String = ctx.get(1).unwrap();
            let network = ipnet::IpNet::from_str(&network_raw)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            network
        },
        3 => {
            let netaddr_raw: String = ctx.get(1).unwrap();
            let netaddr: IpAddr = netaddr_raw.parse()
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            let netmask_raw: String = ctx.get(2).unwrap();
            let netmask: IpAddr = netmask_raw.parse()
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            let network = IpNet::with_netmask(netaddr, netmask)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            network
        },
        n => unreachable!("we only register 2 and 3 arg variants - got {} args", n)
    };

    Ok(network.contains(&ipaddr))
}
