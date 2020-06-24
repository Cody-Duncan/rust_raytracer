use std::ffi::OsStr; // OS string
use std::iter::once;
use std::os::windows::ffi::OsStrExt; // OS String Extended (wide character

pub fn win32_string( value : &str ) -> Vec<u16> {
    OsStr::new( value ).encode_wide().chain( once( 0 ) ).collect()
}