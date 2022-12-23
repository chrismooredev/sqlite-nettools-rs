
use std::{ffi::{CStr, CString}, slice::SliceIndex, net::IpAddr, str::FromStr};
use ipnet::IpNet;
use rusqlite::{ffi, functions::{FunctionFlags, Context}, Connection, types::ValueRef};

pub mod exports;
pub mod oui;
// pub mod vtab_oui;

// #[link(name="ipv4-ext", kind="dylib")]
// extern "C" {
//     fn ip2intFunc(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn int2ipFunc(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn netfrom1Func(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn netfrom2Func(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn netto1Func(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn netto2Func(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn netlength1Func(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn netlength2Func(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn netmasklengthFunc(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn isinnet3Func(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn isinnet2Func(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
//     fn issamenet3Func(context: *mut ffi::sqlite3_context, argc: std::ffi::c_int, argv: *mut *mut ffi::sqlite3_value);
// }


macro_rules! cstr {
    ($s: literal) => {
        unsafe { CStr::from_bytes_with_nul_unchecked(concat!($s, "\0").as_bytes()) }
    }
}

// const FUNCS: &[FunctionDescription] = &[
//     FunctionDescription::new(cstr!("ip2int"),        1, 0, true, ip2intFunc),
//     FunctionDescription::new(cstr!("int2ip"),        1, 0, true, int2ipFunc),
//     FunctionDescription::new(cstr!("netfrom"),       1, 0, true, netfrom1Func),
//     FunctionDescription::new(cstr!("netfrom"),       2, 0, true, netfrom2Func),
//     FunctionDescription::new(cstr!("netto"),         1, 0, true, netto1Func),
//     FunctionDescription::new(cstr!("netto"),         2, 0, true, netto2Func),
//     FunctionDescription::new(cstr!("netlength"),     1, 0, true, netlength1Func),
//     FunctionDescription::new(cstr!("netlength"),     2, 0, true, netlength2Func),
//     FunctionDescription::new(cstr!("netmasklength"), 1, 0, true, netmasklengthFunc),
//     FunctionDescription::new(cstr!("isinnet"),       3, 0, true, isinnet3Func),
//     FunctionDescription::new(cstr!("isinnet"),       2, 0, true, isinnet2Func),
//     FunctionDescription::new(cstr!("issamenet"),     3, 0, true, issamenet3Func),
// ];

fn register_scalar_funcs(dbconn: &Connection) -> rusqlite::Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_INNOCUOUS;
    // eprintln!("scalar funcs: registering INSUBNET (2 args)");
    dbconn.create_scalar_function("INSUBNET", 2, flags, exports::in_subnet)?;
    // eprintln!("scalar funcs: registering INSUBNET (3 args)");
    dbconn.create_scalar_function("INSUBNET", 3, flags, exports::in_subnet)?;

    dbconn.create_scalar_function("MAC_FORMAT",      1, flags, exports::mac::format)?;
    dbconn.create_scalar_function("MAC_FORMAT",      2, flags, exports::mac::format)?;
    dbconn.create_scalar_function("MAC_PREFIX",      1, flags, exports::mac::prefix)?;
    dbconn.create_scalar_function("MAC_MANUF",       1, flags, exports::mac::manuf)?;
    dbconn.create_scalar_function("MAC_MANUFLONG",   1, flags, exports::mac::manuf_long)?;
    dbconn.create_scalar_function("MAC_COMMENT",     1, flags, exports::mac::comment)?;
    dbconn.create_scalar_function("MAC_ISUNICAST",   1, flags, exports::mac::is_unicast)?;
    dbconn.create_scalar_function("MAC_ISMULTICAST", 1, flags, exports::mac::is_multicast)?;
    dbconn.create_scalar_function("MAC_ISUNIVERSAL", 1, flags, exports::mac::is_universal)?;
    dbconn.create_scalar_function("MAC_ISLOCAL",     1, flags, exports::mac::is_local)?;
    // OUIMATCHES

    // didn't end up being meaningfully faster even in tight loops
    // unsafe extern "C" fn mac_manuf_native(ctx: *mut ffi::sqlite3_context, argc: i32, argv: *mut *mut ffi::sqlite3_value) {
    //     let args = std::slice::from_raw_parts(argv, argc as usize);
    //     assert_eq!(args.len(), 1);
    //     assert_eq!(ffi::sqlite3_value_type(args[0]), ffi::SQLITE_TEXT);
        
    //     let text = ffi::sqlite3_value_text(args[0]);
    //     let len = ffi::sqlite3_value_bytes(args[0]);
    //     assert!(
    //         !text.is_null(),
    //         "unexpected SQLITE_TEXT value type with NULL data"
    //     );
    //     let s = std::slice::from_raw_parts(text.cast::<u8>(), len as usize);
    //     let s = std::str::from_utf8(s).unwrap();

    //     let mac: eui48::MacAddress = parse_mac_addr(s).unwrap();

    //     let res = crate::oui::EMBEDDED_DB.search(mac).as_ref().map(oui::OuiMeta::manuf);

    //     match res {
    //         None => { ffi::sqlite3_result_null(ctx); },
    //         Some(s) => {
    //             let len = s.len() as std::ffi::c_int;
    //             let (c_str, destructor) = if len != 0 {
    //                 (s.as_ptr().cast::<std::ffi::c_char>(), ffi::SQLITE_TRANSIENT())
    //             } else {
    //                 // Return a pointer guaranteed to live forever
    //                 ("".as_ptr().cast::<std::ffi::c_char>(), ffi::SQLITE_STATIC())
    //             };

    //             ffi::sqlite3_result_text(ctx, c_str, len, destructor);
    //         }
    //     }
    // }

    // try native one to compare speed
    // let c_name = cstr!("MAC_MANUF_NATIVE");
    // let r = unsafe {
    //     ffi::sqlite3_create_function_v2(
    //         dbconn.handle(),
    //         c_name.as_ptr(),
    //         1,
    //         flags.bits(),
    //         std::ptr::null_mut(),
    //         Some(mac_manuf_native),
    //         None,
    //         None,
    //         None,
    //     )
    // };

    // eprintln!("scalar funcs: done");
    Ok(())
}

#[no_mangle]
unsafe extern "C" fn sqlite3_extension_init(db: *mut ffi::sqlite3, errmsg: *mut *mut std::ffi::c_char, p_api: *const ffi::sqlite3_api_routines) -> std::ffi::c_int {
    // eprintln!("called extension init (db = {:?}, errmsg = {:?}, p_api = {:?})", db, errmsg, p_api);
    rusqlite::ffi::loadable_extension_init(p_api as *mut ffi::sqlite3_api_routines);
    let dbconn = unsafe { rusqlite::Connection::from_handle(db).unwrap() };

    match register_scalar_funcs(&dbconn) {
        Ok(()) => {
            return ffi::SQLITE_OK;
        },
        Err(e) => {
            eprintln!("Unable to register extension functions for sqlite3-inet: {}", e);

            let upper_err = CString::new(e.to_string()).unwrap();

            // This "hack" is due to the custom bindgen for the rust-wrappers for the sqlite3_api_routines not propogating varargs style argument passing
            let api_routine_raw_ptr = core::ptr::addr_of!((*p_api).mprintf);
            let func = api_routine_raw_ptr.read().expect(stringify!(
                "sqlite3_api contains null pointer for mprintf function"
            ));

            *errmsg = (func)("Unable to register extension functions for sqlite3-inet: %s\0".as_ptr() as *const i8, upper_err.as_ptr() as *const i8);

            // SQLITE should de-alloc the memory with sqlite3_free
            return ffi::SQLITE_ERROR;
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseMacError {
    // #[error(transparent)]
    // Length(#[from] eui48::ParseError),
    #[error("MAC address has a bad character length: {0:?}")]
    InvalidLength(String),
    #[error("Found an invalid character in MAC {0:?}: {1:?}")]
    InvalidCharacter(String, char),
}

pub fn parse_mac_addr(s: &str) -> Result<eui48::MacAddress, ParseMacError> {
    parse_mac_addr_extend(s, false)
}
pub fn parse_mac_addr_extend(mut s: &str, zero_extend: bool) -> Result<eui48::MacAddress, ParseMacError> {
    let mut raw = smallstr::SmallString::<[u8; 12]>::new();
    if s.starts_with("0x") { s = &s[2..]; }
    for c in s.chars() {
        if matches!(c, 'A'..='F' | 'a'..='f' | '0'..='9') {
            if raw.len() + 1 > raw.capacity() {
                return Err(ParseMacError::InvalidLength(s.to_owned()));
            }
            raw.push(c);
        } else if ! matches!(c, '-' | '.' | ':') {
            return Err(ParseMacError::InvalidCharacter(s.to_owned(), c));
        }
    }

    if zero_extend {
        const ZEROS: [char; 12] = ['0'; 12];
        raw.extend(&ZEROS[raw.len()..]);
    }

    if raw.len() < 12 {
        return Err(ParseMacError::InvalidLength(s.to_owned()));
    }

    debug_assert_eq!(raw.len(), 12);

    let mac_int: u64 = u64::from_str_radix(raw.as_str(), 16).expect("validated all chars are hexidecimal");

    let mac_raw_long = u64::to_be_bytes(mac_int);
    let mut mac_raw = [0u8; 6];
    mac_raw.copy_from_slice(&mac_raw_long[2..]);

    return Ok(eui48::MacAddress::new(mac_raw))
}

// pub fn parse_mac_addr_patched(s: &str) -> Result<eui48::MacAddress, MacParsingError> {
//     let mut eui: eui48::Eui48 = [0; eui48::EUI48LEN];

//     match s.len() {
//         11..=17 => {}
//         _ => {
//             return Err(eui48::ParseError::InvalidLength(s.len()))?;
//         }
//     }

//     let mut offset = 0;

//     for s in s
//         .split(&[':', '.', '-'][..])
//         .map(|s| if s.starts_with("0x") { &s[2..] } else { s })
//     {
//         let mut hex = 0;
//         let mut i = 0;

//         for c in s.chars() {
//             match c {
//                 '0'..='9' | 'a'..='f' | 'A'..='F' => {
//                     if i % 2 == 1 {
//                         hex <<= 4;
//                     }

//                     hex |= c.to_digit(16).unwrap() as u8;
//                 }
//                 c => return Err(MacParsingError::InvalidCharacter(s.to_owned(), c)),
//             }

//             if i % 2 == 1 {
//                 if offset < eui48::EUI48LEN {
//                     eui[offset] = hex;
//                 }
//                 offset += 1;
//                 hex = 0;
//             }

//             i += 1;
//         }

//         if i % 2 == 1 {
//             if offset < eui48::EUI48LEN {
//                 eui[offset] = hex;
//             }
//             offset += 1;
//         }
//     }

//     if offset != eui48::EUI48LEN {
//         return Err(eui48::ParseError::InvalidByteCount(offset, eui))?;
//     }

//     Ok(eui48::MacAddress::new(eui))
// }

