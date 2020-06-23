extern crate winapi;

use std::io::Error; // Err / Ok
use std::ptr::null_mut; // nullptr

use std::ffi::OsStr; // OS string
use std::iter::once;
use std::mem;
use std::os::windows::ffi::OsStrExt; // OS String Extended (wide character)

use winapi::shared::windef::HWND;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
    MSG,
    WNDCLASSW,
    CS_OWNDC,
    CS_HREDRAW,
    CS_VREDRAW,
    CW_USEDEFAULT,
    WS_OVERLAPPEDWINDOW,
	WS_VISIBLE,
	DefWindowProcW,
    RegisterClassW,
    CreateWindowExW,
    TranslateMessage,
    DispatchMessageW,
    GetMessageW,
};

fn win32_string( value : &str ) -> Vec<u16> {
    OsStr::new( value ).encode_wide().chain( once( 0 ) ).collect()
}

struct Window
{
	handle : HWND,
}

fn create_window() -> Result<Window, Error>
{
	let name = win32_string("sample");
	let title = win32_string("title");

	let style = WS_OVERLAPPEDWINDOW | WS_VISIBLE;

	unsafe 
	{
		let hinstance = GetModuleHandleW( null_mut() );

		let wnd_class = WNDCLASSW 
		{
			style : CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
			lpfnWndProc : Some( DefWindowProcW ),
			hInstance : hinstance,
			lpszClassName : name.as_ptr(),
			cbClsExtra : 0,
			cbWndExtra : 0,
			hIcon: null_mut(),
			hCursor: null_mut(),
			hbrBackground: null_mut(),
			lpszMenuName: null_mut(),
		};

		RegisterClassW(&wnd_class);

		let h_window_handle : HWND = CreateWindowExW(
			0,				// dwExStyle: DWORD
			name.as_ptr(),	// lpClassName: LPCWSTR,
			title.as_ptr(),	// lpWindowName: LPCWSTR,
			style,			// dwStyle: DWORD,
			CW_USEDEFAULT,	// x: c_int,
			CW_USEDEFAULT,	// y: c_int,
			CW_USEDEFAULT,	// nWidth: c_int,
			CW_USEDEFAULT,	// nHeight: c_int,
			null_mut(),		// hWndParent: HWND,
			null_mut(),		// hMenu: HMENU,
			hinstance,		// hInstance: HINSTANCE,
			null_mut(),		// lpParam: LPVOID,
		);

		if h_window_handle.is_null()
		{
			return Err(Error::last_os_error());
		}
		else
		{
			return Ok( Window {handle: h_window_handle} )
		}
	}
}

fn handle_message( window : &mut Window ) -> bool 
{
	let mut message = mem::MaybeUninit::<MSG>::uninit();

	let h_res_get_message = 
		unsafe { GetMessageW(message.as_mut_ptr(), window.handle, 0, 0) };

	if  h_res_get_message > 0 // Valid MSG
	{
		unsafe
		{
			TranslateMessage(message.as_ptr());
			DispatchMessageW(message.as_ptr());
		}
		return true;
	}
	else if h_res_get_message == 0 // WM_QUIT
	{
		return false;
	}
	else // h_res_get_message == -1 // ERROR 
	{
		return false;
	}
}

fn main() 
{
	println!("Hello, Rust!");
	
	let mut window = create_window().unwrap();

	loop 
	{
		if !handle_message( &mut window ) 
		{
            break;
        }
    }
}
