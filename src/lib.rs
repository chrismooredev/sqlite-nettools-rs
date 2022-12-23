
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
    // All of our functions use UTF8 strings, are deterministic, and without side-effects.
    let flags = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_INNOCUOUS;
    dbconn.create_scalar_function("INSUBNET", 2, flags, exports::in_subnet)?;
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

    // eprintln!("scalar funcs: done");
    Ok(())
}

#[no_mangle]
unsafe extern "C" fn sqlite3_extension_init(db: *mut ffi::sqlite3, errmsg: *mut *mut std::ffi::c_char, p_api: *const ffi::sqlite3_api_routines) -> std::ffi::c_int {
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
