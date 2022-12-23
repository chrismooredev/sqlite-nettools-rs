use std::{str::FromStr, num::ParseIntError, collections::BTreeMap, ops::Deref, fmt};

use eui48::{MacAddress, EUI48LEN};

use crate::ParseMacError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OuiMeta<S> {
    short: S,
    long: Option<S>,
    comment: Option<S>,
}
impl<S> OuiMeta<S> {
    pub fn manuf(&self) -> &S {
        &self.short
    }
    pub fn manuf_long(&self) -> Option<&S> {
        self.long.as_ref()
    }
    pub fn comment(&self) -> Option<&S> {
        self.comment.as_ref()
    }
}
impl<'a> OuiMeta<&'a str> {
    pub fn to_owned(&self) -> OuiMeta<String> {
        OuiMeta {
            short: self.short.to_owned(),
            long: self.long.map(|s| s.to_owned()),
            comment: self.comment.map(|s| s.to_owned()),
        }
    }
}
impl OuiMeta<String> {
    pub fn as_ref(&self) -> OuiMeta<&'_ str> {
        OuiMeta {
            short: self.short.as_str(),
            long: self.long.as_ref().map(String::as_str),
            comment: self.comment.as_ref().map(String::as_str),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseOuiError {
    #[error(transparent)]
    MacParsing(#[from] ParseMacError),
    #[error("Unable to parse prefix length from OUI string {1:?}")]
    PrefixLengthParsing(#[source] ParseIntError, String),
    #[error("Parsed an invalid OUI prefix length. Expected values are within range [24, 48]. Got {1} from source prefix {0:?}")]
    PrefixLengthValue(u8, String),
}

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct Oui {
    address: u64,
    length: u8,
}
impl Oui {
    fn mask(&self) -> u64 {
        ((1 << self.length) - 1) << (8*EUI48LEN - self.length as usize)
    }
    pub fn contains(&self, other: &Oui) -> bool {
        // eprintln!("Oui::contains({:?}, {:?} (self mask: {:b}))", self, other, self.mask());
        if self.length > other.length {
            return false;
        }
        other.address & self.mask() == self.address
    }
    pub fn from_addr(mac: MacAddress) -> Oui {
        let mut mac_bytes = [0u8; 8];
        mac_bytes[2..].copy_from_slice(mac.as_bytes());
        let mac_int = u64::from_be_bytes(mac_bytes);
        // eprintln!("\n[src/oui.rs:62] mac={:?}, mac_bytes={:?}, mac_int={:>012x}", mac_bytes, mac, mac_int);
        Oui { address: mac_int, length: 48 }
    }
}
impl FromStr for Oui {
    type Err = ParseOuiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (oui, length) = match s.split_once('/') {
            None => (s, 24),
            Some((oui, slen)) => {
                (
                    oui,
                    slen.parse::<u8>().map_err(|e| ParseOuiError::PrefixLengthParsing(e, s.to_owned()))?
                )
            }
        };

        if length < 24 || length > 48 {
            return Err(ParseOuiError::PrefixLengthValue(length, s.to_owned()))
        }

        let oui_mac = crate::parse_mac_addr_extend(oui, true).unwrap();
        let mut address = Oui::from_addr(oui_mac);
        address.length = length;

        // if dbg {
        //     // eprintln!("[{:?}] macstr={:?}, length={}, mac_int={:>012x}", s, macstr, length, mac_int);
        //     eprintln!("[{:?}] oui={:?}, length={}", s, address, length);
        // }

        Ok(address)
    }
}
impl fmt::Debug for Oui {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mac_bytes_all = u64::to_be_bytes(self.address);
        let mac_bytes = &mac_bytes_all[2..];
        // eprintln!("[src/oui.rs:92] mac_bytes={:?}, oui.address={:>012x}, oui.length={}", mac_bytes, self.address, self.length);
        match self.length {
            24 => f.write_fmt(format_args!(
                "{:02x}:{:02x}:{:02x}",
                mac_bytes[0], mac_bytes[1], mac_bytes[2],
            )),
            _ => f.write_fmt(format_args!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}/{}",
                mac_bytes[0], mac_bytes[1], mac_bytes[2], mac_bytes[3], mac_bytes[4], mac_bytes[5],
                self.length,
            )),
        }
    }
}

pub struct OuiDb(Vec<(Oui, OuiMeta<String>)>);

lazy_static::lazy_static! {
    pub static ref EMBEDDED_DB: OuiDb = {
        OuiDb::parse_from_string(OuiDb::WIRESHARK_OUI_DB_EMBEDDED).expect("failure parsing embedded wireshark oui database")
    };
}

// #[derive(Debug, thiserror::Error)]
// #[error("")]
// pub struct DbParsingError {
//     line: usize,
//     #[source]
//     error: DbParsingErrorType
// }

#[derive(Debug, thiserror::Error)]
pub enum DbParsingError {
    #[error("error parsing oui in db record (line {0}: {2:?})")]
    OuiParsing(usize, #[source] ParseOuiError, String),
    #[error("invalid number of fields in oui db record, expected [2, 4] got {1} (line {0}: {2:?})")]
    BadFieldCount(usize, usize, String),
    
    #[cfg(debug_assertions)]
    #[error("entries with duplicate prefix's exist within the OUI database")]
    DuplicatedEntries,
}

impl OuiDb {
    /// The latest copy of Wireshark's OUI database at compile time.
    /// 
    /// Latest copy is available here: https://gitlab.com/wireshark/wireshark/raw/master/manuf
    pub const WIRESHARK_OUI_DB_EMBEDDED: &str = include_str!(concat!(env!("OUT_DIR"), "/wireshark_oui_db.txt"));

    // TODO: pub fn parse_from_reader<R: BufRead>(txt: R) -> Result<OuiDb, DbParsingError>

    /// Parse a file in the format of Wireshark's OUI database into memory.
    /// 
    /// Wireshark's reference OUI database can be found here: https://gitlab.com/wireshark/wireshark/raw/master/manuf
    /// 
    /// Essentially a straight port of the python 'manuf' library's parsing
    pub fn parse_from_string(txt: &str) -> Result<OuiDb, DbParsingError> {
        let mut v: Vec<(Oui, OuiMeta<String>)> = txt.split('\n')
            .enumerate()
            .map(|(lnum, l)| (lnum, l.trim()))
            .filter(|(_, l)| {
                l.len() > 0 && !l.starts_with('#')
            })
            .map(|(lnum, l)| {
                let mut _fields = [""; 8];
                let fields: &[&str] = {
                    let mut len = 0;
                    l.split('\t').filter(|f| f.len() > 1)
                        .enumerate()
                        .for_each(|(i, part)| {
                            len = i+1;
                            _fields[i] = part.trim();
                        });
                    &_fields[..len]
                };
                if ! (2..=4).contains(&fields.len()) {
                    return Err(DbParsingError::BadFieldCount(lnum, fields.len(), l.to_owned()));
                }
                let ouispec: Oui = fields[0].parse()
                    .map_err(|e| DbParsingError::OuiParsing(lnum, e, l.to_owned()))?;
                let short = fields[1];
                let long = fields.get(2).copied();
                let comment = fields.get(3).map(|s| s.trim_matches('#').trim());
                Ok((ouispec, OuiMeta { short, long, comment }.to_owned()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // sort it for binary searching later
        v.sort_by_key(|(k, _v)| *k);

        #[cfg(debug_assertions)] {
            // no need to error on this if running in release mode
            // the sourced DB shouldn't have any and it's not worth erroring over anyway
            // this is primarily for diagnostics
            let prededup_len = v.len();
            v.dedup_by_key(|(k, _v)| *k);
            if prededup_len != v.len() {
                return Err(DbParsingError::DuplicatedEntries);
            }
        }

        let dbg_str: String = v.iter()
            .enumerate()
            .map(|(i, (o, om))| format!("{:>05}\t{:>012x}/{}\t{:?}\t{:?}\n", i, o.address, o.length, o, om))
            .collect();
        std::fs::write("oui_db_dump2.txt", dbg_str).unwrap();

        return Ok(OuiDb(v));
    }

    pub fn search_entry(&self, mac: MacAddress) -> Option<(Oui, OuiMeta<&str>)> {
        let as_oui = Oui::from_addr(mac);
        // eprintln!("searching MAC {:?} with OUI {:?}", mac, as_oui);
        let base_i = match self.0.binary_search_by_key(&as_oui, |(o, om)| *o) {
            Ok(i) => i, // exact match
            Err(i) => {
                // should be n-above our desired entry
                // should /be/ our desired entry if the prefix is long
                // may have to iterate towards zero if we are within a longer prefix, and must match for the parent prefix
                // subtract zero to go to the lower end of our match
                i-1
            },
        };
        let mut i = base_i;

        loop {
            let (o, om) = self.0.get(i)?;
            if o.contains(&as_oui) {
                // this is our prefix
                return Some((*o, om.as_ref()));
            } else if ! o.contains(&as_oui) && o.length <= 24 {
                // we reached a top-level-prefix (/24) that doesn't contain us - we have none
                return None;
            } else {
                // continue searching upwards for a containing prefix until we find one, or find a top-level that we arent' in
                i -= 1;
            }
        }
    }

    pub fn raw_prefixes(&self) -> impl Iterator<Item = (Oui, OuiMeta<&str>)> {
        self.0.iter()
            .map(|(o, om)| (*o, om.as_ref()))
    }
    pub fn search_prefix(&self, mac: MacAddress) -> Option<Oui> {
        self.search_entry(mac).map(|(p, _)| p)
    }
    pub fn search(&self, mac: MacAddress) -> Option<OuiMeta<&str>> {
        self.search_entry(mac).map(|(_, om)| om)
    }

}

#[test]
fn embedded_db_builds() {
    OuiDb::parse_from_string(OuiDb::WIRESHARK_OUI_DB_EMBEDDED).unwrap();
}

#[test]
fn match_no_long_name() {
    // 00:00:17	Oracle
    let mac = crate::parse_mac_addr("00:00:17:aa:bb:cc").unwrap();
    assert_eq!(EMBEDDED_DB.search(mac),
        Some(OuiMeta {
            short: "Oracle",
            long: None,
            comment: None,
        })
    );
}

#[test]
fn match_prefix_zeros() {
    // 00:00:17	Oracle
    let mac = crate::parse_mac_addr("00:00:00:00:00:00").unwrap();
    assert_eq!(EMBEDDED_DB.search(mac),
        Some(OuiMeta {
            short: "00:00:00",
            long: Some("Officially Xerox, but 0:0:0:0:0:0 is more common"),
            comment: None,
        })
    );
}

#[test]
fn match_prefix_exact() {
    // 2C:23:3A	HewlettP	Hewlett Packard
    let mac = crate::parse_mac_addr("2c:23:3a:00:00:00").unwrap();
    assert_eq!(EMBEDDED_DB.search(mac),
        Some(OuiMeta {
            short: "HewlettP",
            long: Some("Hewlett Packard"),
            comment: None,
        })
    );
}

#[test]
fn match_prefix_basic() {
    // 2C:23:3A	HewlettP	Hewlett Packard
    let mac = crate::parse_mac_addr("2c:23:3a:aa:bb:cc").unwrap();
    assert_eq!(EMBEDDED_DB.search(mac),
        Some(OuiMeta {
            short: "HewlettP",
            long: Some("Hewlett Packard"),
            comment: None,
        })
    );
}


#[test]
fn match_prefix_extended() {
    // 8C:47:6E:30:00:00/28	Shanghai	Shanghai Satellite Communication Technology Co.,Ltd
    let mac = crate::parse_mac_addr("8c:47:6e:3a:bb:cc").unwrap();
    assert_eq!(EMBEDDED_DB.search(mac),
        Some(OuiMeta {
            short: "Shanghai",
            long: Some("Shanghai Satellite Communication Technology Co.,Ltd"),
            comment: None,
        })
    );
}

#[test]
fn match_commented() {
    // 08:00:87	XyplexTe	Xyplex	# terminal servers
    let mac = crate::parse_mac_addr("08:00:87:aa:bb:cc").unwrap();
    assert_eq!(EMBEDDED_DB.search(mac),
        Some(OuiMeta {
            short: "XyplexTe",
            long: Some("Xyplex"),
            comment: Some("terminal servers"),
        })
    );
}

#[test]
fn match_unicode() {
    // 8C:1F:64:CB:20:00/36	DyncirSo	Dyncir Soluções Tecnológicas Ltda
    let mac = crate::parse_mac_addr("8c:1f:64:cb:2b:cc").unwrap();
    assert_eq!(EMBEDDED_DB.search(mac),
        Some(OuiMeta {
            short: "DyncirSo",
            long: Some("Dyncir Soluções Tecnológicas Ltda"),
            comment: None,
        })
    );
}

#[test]
fn resolve_mac_to_superprefix_when_missing_subprefix() {
    // 2C:27:9E	IEEERegi	IEEE Registration Authority
    // is split into /28, without a 2C:27:9E:F0:00:00/28 member
    let mac = crate::parse_mac_addr("2c:27:9e:fa:bb:cc").unwrap();
    assert_eq!(EMBEDDED_DB.search(mac),
        Some(OuiMeta {
            short: "IEEERegi",
            long: Some("IEEE Registration Authority"),
            comment: None,
        })
    );
}

#[test]
fn match_none() {
    // B0:C5:59	SamsungE	Samsung Electronics Co.,Ltd
    // B0:C5:CA	IEEERegi	IEEE Registration Authority
    let mac = crate::parse_mac_addr("b0:c5:5a:aa:bb:cc").unwrap();
    assert_eq!(EMBEDDED_DB.search(mac),
        None
    );
}
