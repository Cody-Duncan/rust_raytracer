use std::io::Error; // Err / Ok
use std::ptr::null_mut; // nullptr

use winapi::shared::windef::HWND;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
    WNDCLASSW,
    CS_OWNDC,
    CS_HREDRAW,
    CS_VREDRAW,
	CW_USEDEFAULT,
	SW_SHOW,
    WS_OVERLAPPEDWINDOW,
	WS_VISIBLE,
    RegisterClassW,
	CreateWindowExW,
	ShowWindow,
	UpdateWindow
};

use crate::win_utilities::win32_string;
use crate::win_platform::{ window_proc };

#[derive(Copy, Clone)]
pub struct Window
{
	pub handle : HWND,
}
unsafe impl std::marker::Send for Window {}



pub fn create_window() -> Result<Window, Error>
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
			lpfnWndProc : Some(window_proc),
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
			1280,			// nWidth: c_int,
			720,			// nHeight: c_int,
			null_mut(),		// hWndParent: HWND,
			null_mut(),		// hMenu: HMENU,
			hinstance,		// hInstance: HINSTANCE,
			null_mut(),		// lpParam: LPVOID,
		);

		println!("{}", h_window_handle as i32);

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

pub fn show_window( window : Window )
{
	unsafe
	{
		ShowWindow(window.handle, SW_SHOW);
		UpdateWindow(window.handle);
	}
}