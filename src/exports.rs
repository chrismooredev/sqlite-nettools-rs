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
    use smallstr::SmallString;

    use crate::{
        mac::MacStyle,
        oui::{Oui, OuiMeta},
    };

    #[derive(thiserror::Error, Debug)]
    enum MacFormatError {
        #[error("Mixed case format specifier is not allowed. Input case is used to determine output casing.")]
        MixedCaseFmtSpecifier,
        #[error("Bad format specifier provided (got {0:?}). Omit format specifier, or provide one of the following: (NULL, `hex`, `hexstring`), `hexadecimal`, `bare`, `dot`, `canonical`, `interface-id`, `link-local`)")]
        BadFmtSpecifier(String),
    }

    fn find_mac(
        ctx: &rusqlite::functions::Context<'_>,
    ) -> rusqlite::Result<Option<(Oui, OuiMeta<&'static str>)>> {
        let Some(s) = ctx.get_raw(0).as_str_or_null()? else { return Ok(None); };
        if s.is_empty() {
            return Ok(None);
        }
        let mac = crate::oui::parse_mac_addr(s)
            .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

        Ok(crate::oui::EMBEDDED_DB.search_entry(mac))
    }

    /// # MAC_FORMAT(mac, \[NULL|fmt]) -> mac'
    /// Formats a MAC address into a normalized form. Uses `hexstring` format by default.
    ///
    /// The casing of the format string determines the casing of the output. Mixed-case output is not supported.
    ///
    /// Note that prefixing the fmt string with a tilde `~` will make the function use the `hex` format
    /// when a format is not otherwise found. This can be used to prevent query errors for an invalid
    /// format type.
    ///
    /// Prefixing the format string with an question mark `?` will make the function emit NULL on a bad MAC address.
    /// Similarly, this can be used to prevent a query error on a bad MAC address. This effectively allows the function
    /// to be used to validate a MAC address.
    ///
    /// The format and MAC address validation flags `~`/`?` can be intermixed, and they can be repeated. (Additional flags have no effect)
    ///
    /// # Usage
    /// |Call|Result|
    /// |-|-|
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff')`                 | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', NULL)`           | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'hex')`          | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'HEX')`          | `'AA:BB:CC:DD:EE:FF'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'hexstring')`    | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'colon')`        | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'hexadecimal')`  | `'0xaabbccddeeff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'bare')`         | `'aabbccddeeff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'dot')`          | `'aabb.ccdd.eeff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'dash')`         | `'aa-bb-cc-dd-ee-ff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'canonical')`    | `'aa-bb-cc-dd-ee-ff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'interface-id')` | `'a8bb:ccff:fedd:eeff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'link-local')`   | `'fe80::a8bb:ccff:fedd:eeff'` |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', 'de$H')`         | N/A - A query error is raised with an appropriate error message |
    /// |`MAC_FORMAT('aa-bb-cc-dd-ee-ff', '~de$H')`        | `'aa:bb:cc:dd:ee:ff'` |
    /// |`MAC_FORMAT('a!-bbkcc-dd2ee-ff', '?dash')`        | `NULL` |
    /// |`MAC_FORMAT('a!-bbcc-dd2ee-ff', '?~')`            | `NULL` |
    pub fn format(ctx: &rusqlite::functions::Context<'_>) -> rusqlite::Result<Option<String>> {
        let mac_str = ctx.get_raw(0).as_str()?;

        let mut raw_fmt = (ctx.len() == 2)
            .then(|| ctx.get_raw(1).as_str_or_null())
            .transpose()?
            .flatten();
        let mut has_upper = false;
        let mut use_default_on_bad_fmt = false;
        let mut ret_null_on_bad_mac = false;
        if let Some(fmt) = raw_fmt.as_mut() {
            loop {
                if fmt.starts_with('~') {
                    *fmt = &fmt[1..];
                    use_default_on_bad_fmt = true;
                } else if fmt.starts_with('?') {
                    *fmt = &fmt[1..];
                    ret_null_on_bad_mac = true;
                } else {
                    break;
                }
            }
            has_upper = fmt.contains(|c: char| c.is_ascii_uppercase());
            let has_lower = fmt.contains(|c: char| c.is_ascii_lowercase());
            if has_upper && has_lower && !use_default_on_bad_fmt {
                return Err(rusqlite::Error::UserFunctionError(Box::new(
                    MacFormatError::MixedCaseFmtSpecifier,
                )));
            }
        }

        let mut style = MacStyle::Colon;
        if let Some(fmt) = raw_fmt {
            let mut fmt = SmallString::<[u8; 16]>::from_str(fmt);
            fmt.make_ascii_lowercase();
            style = match fmt.as_str() {
                "" | "hex" | "hexstring" | "colon" => MacStyle::Colon,
                "hexadecimal" => MacStyle::Prefixed,
                "bare" => MacStyle::Plain,
                "dot" => MacStyle::Dots,
                "dash" | "canonical" => MacStyle::Dashed,
                "interface-id" => MacStyle::InterfaceId,
                "link-local" => MacStyle::LinkLocal,
                _ if use_default_on_bad_fmt => style, // passthru default
                _ => {
                    return Err(rusqlite::Error::UserFunctionError(Box::new(
                        MacFormatError::BadFmtSpecifier(raw_fmt.unwrap().to_string()),
                    )))
                }
            };
        }

        let mac = match crate::oui::parse_mac_addr(mac_str) {
            Ok(m) => m,
            Err(_) if ret_null_on_bad_mac => return Ok(None),
            Err(e) => return Err(rusqlite::Error::UserFunctionError(Box::new(e))),
        };

        Ok(Some(style.format(mac, has_upper).to_string()))
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
        Ok(mac.map(|(oui, _om)| format!("{oui:?}")))
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
    pub fn manuf_long(
        ctx: &rusqlite::functions::Context<'_>,
    ) -> rusqlite::Result<Option<&'static str>> {
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
    pub fn comment(
        ctx: &rusqlite::functions::Context<'_>,
    ) -> rusqlite::Result<Option<&'static str>> {
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

            let mac = crate::oui::parse_mac_addr(&mac_str)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            Ok(Some(mac.$fname()))
        }};
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
    let ipaddr: IpAddr = ipaddr_raw
        .parse()
        .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

    let network = match args {
        2 => {
            let network_raw: String = ctx.get(1).unwrap();
            ipnet::IpNet::from_str(&network_raw)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))
        }
        3 => {
            let netaddr_raw: String = ctx.get(1).unwrap();
            let netaddr: IpAddr = netaddr_raw
                .parse()
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            let netmask_raw: String = ctx.get(2).unwrap();
            let netmask: IpAddr = netmask_raw
                .parse()
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
            IpNet::with_netmask(netaddr, netmask)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))
        }
        n => unreachable!("we only register 2 and 3 arg variants - got {} args", n),
    };

    Ok(network?.contains(&ipaddr))
}
