
use std::ffi::CString;
use rusqlite::{ffi, functions::FunctionFlags, Connection};

/// Main collection of functions exported to SQLite. Also acts as documentation for those functions.
pub mod exports;

/// Non-alloc MAC address formatting
pub mod mac;

/// OUI database and lookup
pub mod oui;

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

    // eprintln!("scalar funcs: done");
    Ok(())
}

#[no_mangle]
unsafe extern "C" fn sqlite3_extension_init(db: *mut ffi::sqlite3, errmsg: *mut *mut std::ffi::c_char, p_api: *const ffi::sqlite3_api_routines) -> std::ffi::c_int {
    rusqlite::ffi::loadable_extension_init(p_api as *mut ffi::sqlite3_api_routines);
    let dbconn = unsafe { rusqlite::Connection::from_handle(db).unwrap() };

    match register_scalar_funcs(&dbconn) {
        Ok(()) => {
            ffi::SQLITE_OK
        },
        Err(e) => {
            eprintln!("Unable to register extension functions for sqlite3-inet: {e}");

            let upper_err = CString::new(e.to_string()).unwrap();

            // This "hack" is due to the custom bindgen for the rust-wrappers for the sqlite3_api_routines not propogating varargs style argument passing
            let api_routine_raw_ptr = core::ptr::addr_of!((*p_api).mprintf);
            let func = api_routine_raw_ptr.read().expect(stringify!(
                "sqlite3_api contains null pointer for mprintf function"
            ));

            *errmsg = (func)("Unable to register extension functions for sqlite3-inet: %s\0".as_ptr() as *const i8, upper_err.as_ptr() as *const i8);

            // SQLITE should de-alloc the memory with sqlite3_free
            ffi::SQLITE_ERROR
        }
    }
}
