
use eui48::MacAddress;
use smallstr::SmallString;

use crate::oui::Oui;

#[derive(Clone, Copy)]
pub enum MacStyle {
    Plain,
    Dashed,
    Colon,
    Dots,
    Prefixed,
    InterfaceId,
    LinkLocal,
}
impl MacStyle {
    const BASE_PLAIN:      [u8; 25] = *b"############@@@@@@@@@@@@@";
    const BASE_DASHED:     [u8; 25] = *b"##-##-##-##-##-##@@@@@@@@";
    const BASE_COLON:      [u8; 25] = *b"##:##:##:##:##:##@@@@@@@@";
    const BASE_DOTS:       [u8; 25] = *b"####.####.####@@@@@@@@@@@";
    const BASE_PREFIXED:   [u8; 25] = *b"0x############@@@@@@@@@@@";
    const BASE_INTF_ID:    [u8; 25] = *b"####:##ff:fe##:####@@@@@@";
    const BASE_LINK_LOCAL: [u8; 25] = *b"fe80::####:##ff:fe##:####";

    const OFFSETS_NONE: [(usize, usize); 2*6] = [
        ( 0, 0x2c),
        ( 1, 0x28),
        ( 2, 0x24),
        ( 3, 0x20),
        ( 4, 0x1c),
        ( 5, 0x18),
        ( 6, 0x14),
        ( 7, 0x10),
        ( 8, 0x0c),
        ( 9, 0x08),
        (10, 0x04),
        (11, 0x00),
    ];
    const OFFSETS_NONE_PREFIXED: [(usize, usize); 2*6] = [
        ( 2, 0x2c),
        ( 3, 0x28),
        ( 4, 0x24),
        ( 5, 0x20),
        ( 6, 0x1c),
        ( 7, 0x18),
        ( 8, 0x14),
        ( 9, 0x10),
        (10, 0x0c),
        (11, 0x08),
        (12, 0x04),
        (13, 0x00),
    ];
    const OFFSETS_BYTE: [(usize, usize); 2*6] = [
        ( 0, 0x2c),
        ( 1, 0x28),
        ( 3, 0x24),
        ( 4, 0x20),
        ( 6, 0x1c),
        ( 7, 0x18),
        ( 9, 0x14),
        (10, 0x10),
        (12, 0x0c),
        (13, 0x08),
        (15, 0x04),
        (16, 0x00),
    ];
    const OFFSETS_SHORT: [(usize, usize); 2*6] = [
        ( 0, 0x2c),
        ( 1, 0x28),
        ( 2, 0x24),
        ( 3, 0x20),
        ( 5, 0x1c),
        ( 6, 0x18),
        ( 7, 0x14),
        ( 8, 0x10),
        (10, 0x0c),
        (11, 0x08),
        (12, 0x04),
        (13, 0x00),
    ];
    const OFFSETS_INTF_ID: [(usize, usize); 2*6] = [
        ( 0, 0x2c),
        ( 1, 0x28),
        ( 2, 0x24),
        ( 3, 0x20),
        ( 5, 0x1c),
        ( 6, 0x18),
        (12, 0x14),
        (13, 0x10),
        (15, 0x0c),
        (16, 0x08),
        (17, 0x04),
        (18, 0x00),
    ];
    const OFFSETS_LINK_LOCAL: [(usize, usize); 2*6] = [
        ( 6, 0x2c),
        ( 7, 0x28),
        ( 8, 0x24),
        ( 9, 0x20),
        (11, 0x1c),
        (12, 0x18),
        (18, 0x14),
        (19, 0x10),
        (21, 0x0c),
        (22, 0x08),
        (23, 0x04),
        (24, 0x00),
    ];

    #[inline(always)]
    const fn output_size(&self) -> usize {
        match self {
            MacStyle::Plain => 12,
            MacStyle::Dashed => 17,
            MacStyle::Colon => 17,
            MacStyle::Dots => 14,
            MacStyle::Prefixed => 14,
            MacStyle::InterfaceId => 19,
            MacStyle::LinkLocal => 25,
        }
    }

    #[inline(always)]
    const fn base(&self) -> [u8; 25] {
        match self {
            MacStyle::Plain => MacStyle::BASE_PLAIN,
            MacStyle::Dashed => MacStyle::BASE_DASHED,
            MacStyle::Colon => MacStyle::BASE_COLON,
            MacStyle::Dots => MacStyle::BASE_DOTS,
            MacStyle::Prefixed => MacStyle::BASE_PREFIXED,
            MacStyle::InterfaceId => MacStyle::BASE_INTF_ID,
            MacStyle::LinkLocal => MacStyle::BASE_LINK_LOCAL,
        }
    }

    #[inline(always)]
    const fn offsets(&self) -> [(usize, usize); 2*6] {
        match self {
            MacStyle::Plain => MacStyle::OFFSETS_NONE,
            MacStyle::Dashed => MacStyle::OFFSETS_BYTE,
            MacStyle::Colon => MacStyle::OFFSETS_BYTE,
            MacStyle::Dots => MacStyle::OFFSETS_SHORT,
            MacStyle::Prefixed => MacStyle::OFFSETS_NONE_PREFIXED,
            MacStyle::InterfaceId => MacStyle::OFFSETS_INTF_ID,
            MacStyle::LinkLocal => MacStyle::OFFSETS_LINK_LOCAL,
        }
    }

    #[inline(always)]
    pub(crate) const fn _format_mac<const UPPERCASE: bool>(eui64: u64, offsets: [(usize, usize); 12], mut arr: [u8; 25]) -> [u8; 25] {
        let nibbles: [u8; 16] = if UPPERCASE {
            *b"0123456789ABCDEF"
        } else {
            *b"0123456789abcdef"
        };
        let eui = eui64 as usize;
        let mut i = 0;
        while i < offsets.len() {
            let (ind, off) = offsets[i];
            arr[ind] = nibbles[(eui >> off) & 0xf];
            i += 1;
        }
        arr
    }

    #[inline(always)]
    pub fn format(&self, mac: MacAddress, uppercase: bool) -> SmallString<[u8; 25]> {
        let mut as_u64 = Oui::from_addr(mac).as_int();
        if matches!(self, MacStyle::InterfaceId | MacStyle::LinkLocal) {
            as_u64 ^= 0x0000_0200_0000_0000;
        }

        let mut fmtd = match uppercase {
            true  => MacStyle::_format_mac::<true >(as_u64, self.offsets(), self.base()),
            false => MacStyle::_format_mac::<false>(as_u64, self.offsets(), self.base()),
        };

        if uppercase && matches!(self, MacStyle::InterfaceId | MacStyle::LinkLocal) {
            // ensure the fe80:: prefix and ff:fe internal bytes are capitalized
            fmtd.make_ascii_uppercase();
        }

        let fmtd_trimmed = &fmtd[..self.output_size()];

        let as_str = if cfg!(debug_assertions) {
            std::str::from_utf8(fmtd_trimmed).expect("found invalid utf8 in formatted MAC address??")
        } else {
            // SAFETY:
            // All base strings are valid ascii/1-byte UTF8 codepoints
            // any modifications to the base strings are from a byte buffer of ascii (1-byte) characters
            // so any characters in the resulting fmtd string are 1-byte characters of valid UTF8
            unsafe {
                std::str::from_utf8_unchecked(fmtd_trimmed)
            }
        };
        SmallString::from_str(as_str)
    }
}


pub fn format_mac_dashed(mac: MacAddress) -> SmallString<[u8; 25]> {
    MacStyle::Plain.format(mac, true)
}

#[test]
fn style_formatting() {
    let mac = Oui::from_int(0x0000AABBCCDDEEFF).unwrap().as_mac();
    assert_eq!("aabbccddeeff", MacStyle::Plain.format(mac, false).as_str());
    assert_eq!("aa-bb-cc-dd-ee-ff", MacStyle::Dashed.format(mac, false).as_str());
    assert_eq!("a8bb:ccff:fedd:eeff", MacStyle::InterfaceId.format(mac, false).as_str());
    assert_eq!("fe80::a8bb:ccff:fedd:eeff", MacStyle::LinkLocal.format(mac, false).as_str());
    assert_eq!("FE80::A8BB:CCFF:FEDD:EEFF", MacStyle::LinkLocal.format(mac, true).as_str());
}
