use eui48::MacAddress;
use smallstr::SmallString;

use crate::oui::Oui;

#[derive(Clone, Copy, Debug)]
pub enum MacStyle {
    Plain,
    Dashed,
    Colon,
    Dots,
    Prefixed,
    InterfaceId,
    LinkLocal,
}

struct StyleDescription {
    base: [u8; 25],
    length: usize,
    offsets: [usize; 12],
}

macro_rules! style_desc {
    ($style: ident, $base: ident, $len: expr, $offset: ident) => {
        (MacStyle::$style, StyleDescription {
            base: MacStyle::$base,
            length: $len,
            offsets: MacStyle::$offset,
        })
    }
}

impl MacStyle {
    const NIBBLE_IDXS: [usize; 12] = [0x2c, 0x28, 0x24, 0x20, 0x1c, 0x18, 0x14, 0x10, 0x0c, 0x08, 0x04, 0x00];

    const BASE_PLAIN:      [u8; 25] = *b"############@@@@@@@@@@@@@";
    const BASE_DASHED:     [u8; 25] = *b"##-##-##-##-##-##@@@@@@@@";
    const BASE_COLON:      [u8; 25] = *b"##:##:##:##:##:##@@@@@@@@";
    const BASE_DOTS:       [u8; 25] = *b"####.####.####@@@@@@@@@@@";
    const BASE_PREFIXED:   [u8; 25] = *b"0x############@@@@@@@@@@@";
    const BASE_INTF_ID:    [u8; 25] = *b"####:##ff:fe##:####@@@@@@";
    const BASE_LINK_LOCAL: [u8; 25] = *b"fe80::####:##ff:fe##:####";

    const OFFSETS_NONE: [usize; 2*6] = [0,1,2,3,4,5,6,7,8,9,10,11];
    const OFFSETS_NONE_PREFIXED: [usize; 2*6] = [2,3,4,5,6,7,8,9,10,11,12,13];
    const OFFSETS_BYTE: [usize; 2*6] = [0,1,3,4,6,7,9,10,12,13,15,16];
    const OFFSETS_SHORT: [usize; 2*6] = [0,1,2,3,5,6,7,8,10,11,12,13];
    const OFFSETS_INTF_ID: [usize; 2*6] = [0,1,2,3,5,6,12,13,15,16,17,18];
    const OFFSETS_LINK_LOCAL: [usize; 2*6] = [6,7,8,9,11,12,18,19,21,22,23,24];

    const FMT_TABLE: &'static [(MacStyle, StyleDescription)] = &[
        style_desc!(Plain, BASE_PLAIN, 12, OFFSETS_NONE),
        style_desc!(Dashed, BASE_DASHED, 17, OFFSETS_BYTE),
        style_desc!(Colon, BASE_COLON, 17, OFFSETS_BYTE),
        style_desc!(Dots, BASE_DOTS, 14, OFFSETS_SHORT),
        style_desc!(Prefixed, BASE_PREFIXED, 14, OFFSETS_NONE_PREFIXED),
        style_desc!(InterfaceId, BASE_INTF_ID, 19, OFFSETS_INTF_ID),
        style_desc!(LinkLocal, BASE_LINK_LOCAL, 25, OFFSETS_LINK_LOCAL),
    ];

    #[inline(always)]
    const fn fmt_desc(&self) -> &'static StyleDescription {
        match self {
            MacStyle::Plain => &MacStyle::FMT_TABLE[0].1,
            MacStyle::Dashed => &MacStyle::FMT_TABLE[1].1,
            MacStyle::Colon => &MacStyle::FMT_TABLE[2].1,
            MacStyle::Dots => &MacStyle::FMT_TABLE[3].1,
            MacStyle::Prefixed => &MacStyle::FMT_TABLE[4].1,
            MacStyle::InterfaceId => &MacStyle::FMT_TABLE[5].1,
            MacStyle::LinkLocal => &MacStyle::FMT_TABLE[6].1,
        }
    }

    /// The length of a MAC address when serialized into a string
    #[inline(always)]
    pub const fn length(&self) -> usize {
        self.fmt_desc().length
    }

    /// A template string of a MAC address. Only the first `MacStyle::length()` bytes will be used, the rest is padding.
    #[inline(always)]
    pub const fn base(&self) -> [u8; 25] {
        self.fmt_desc().base
    }

    #[inline(always)]
    pub(crate) const fn _format_mac<const UPPERCASE: bool>(
        eui64: u64,
        offsets: [usize; 12],
        mut arr: [u8; 25],
    ) -> [u8; 25] {
        let nibbles: [u8; 16] = if UPPERCASE {
            *b"0123456789ABCDEF"
        } else {
            *b"0123456789abcdef"
        };
        let eui = eui64 as usize;
        let mut i = 0;
        while i < offsets.len() {
            let ind = offsets[i];
            let off = MacStyle::NIBBLE_IDXS[i];
            arr[ind] = nibbles[(eui >> off) & 0xf];
            i += 1;
        }
        arr
    }

    /// Formats a MAC address into a small string of at most 25 bytes.
    pub fn format(&self, mac: MacAddress, uppercase: bool) -> SmallString<[u8; 25]> {
        let (fmtd, len) = self.format_internal(mac.as_bytes().try_into().unwrap(), uppercase);

        let fmtd_trimmed = &fmtd[..len];

        let as_str = if cfg!(debug_assertions) {
            match std::str::from_utf8(fmtd_trimmed) {
                Ok(s) => s,
                Err(e) => panic!("found invalid utf8 in freshly formatted MAC address: {:?}", e),
            }
        } else {
            // SAFETY:
            // All base strings are valid ascii/1-byte UTF8 codepoints
            // any modifications to the base strings are from a byte buffer of ascii (1-byte) characters
            // so any characters in the resulting fmtd string are 1-byte characters of valid UTF8
            unsafe { std::str::from_utf8_unchecked(fmtd_trimmed) }
        };

        // waiting for any kind of const support from SmallString
        SmallString::from_str(as_str)
    }

    /// An const version of `MacStyle::format`. Returns a byte buffer, with a string length.
    /// 
    /// For use in a const context, the function omits:
    /// - Trimming output to formatted length: see `MacStyle::length`, or the second value in the returned tuple
    /// - UTF8 validity: While the trimmed output should always be UTF8, it is not checked in this function.
    /// 
    /// # Example
    /// ```
    /// # use sqlite3_nettools::mac::MacStyle;
    /// let style = MacStyle::Colon;
    /// let (raw, len) = style.format_internal([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF], false);
    /// let trimmed = &raw[..len];
    /// assert_eq!(trimmed, b"aa:bb:cc:dd:ee:ff");
    /// # assert_eq!(style.length(), len);
    /// ```
    #[inline(always)]
    pub const fn format_internal(&self, mac: [u8; 6], uppercase: bool) -> ([u8; 25], usize) {
        let mut as_u64 = Oui::from_array(mac).as_int();
        if matches!(self, MacStyle::InterfaceId | MacStyle::LinkLocal) {
            as_u64 ^= 0x0000_0200_0000_0000;
        }

        let style = self.fmt_desc();
        let mut fmtd = match uppercase {
            true  => MacStyle::_format_mac::<true >(as_u64, style.offsets, style.base),
            false => MacStyle::_format_mac::<false>(as_u64, style.offsets, style.base),
        };

        if uppercase && matches!(self, MacStyle::InterfaceId | MacStyle::LinkLocal) {
            // ensure the fe80:: prefix and ff:fe internal bytes are capitalized
            let mut i = 0;
            while i < fmtd.len() {
                if fmtd[i].is_ascii_lowercase() {
                    fmtd[i] = fmtd[i].to_ascii_uppercase();
                }
                i += 1;
            }

            // above version is const
            // fmtd.make_ascii_uppercase();
        }

        (fmtd, style.length)
    }
}

pub fn format_mac_dashed(mac: MacAddress) -> SmallString<[u8; 25]> {
    MacStyle::Plain.format(mac, true)
}

#[test]
fn style_formatting() {
    let mac = Oui::from_int(0x0000AABBCCDDEEFF).unwrap().as_mac();
    assert_eq!("aabbccddeeff", MacStyle::Plain.format(mac, false).as_str());
    assert_eq!(
        "aa-bb-cc-dd-ee-ff",
        MacStyle::Dashed.format(mac, false).as_str()
    );
    assert_eq!(
        "a8bb:ccff:fedd:eeff",
        MacStyle::InterfaceId.format(mac, false).as_str()
    );
    assert_eq!(
        "fe80::a8bb:ccff:fedd:eeff",
        MacStyle::LinkLocal.format(mac, false).as_str()
    );
    assert_eq!(
        "FE80::A8BB:CCFF:FEDD:EEFF",
        MacStyle::LinkLocal.format(mac, true).as_str()
    );
}
