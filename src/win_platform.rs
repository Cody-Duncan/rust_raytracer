use std::error::Error;
use std::fmt;
use std::mem;
use std::result::Result;
use std::string::String;
use std::sync::mpsc;

use winapi::um::winuser::{
	MSG,
	PM_REMOVE,
	WM_QUIT,
	WM_CLOSE,
	WM_DESTROY,
    TranslateMessage,
    DispatchMessageW,
	PeekMessageW,
};

use winapi::shared::windef::
{
	HWND,
};

use winapi::shared::minwindef::
{
	UINT,
	LPARAM,
	WPARAM,
	LRESULT
};

use crate::win_window;

#[derive(Debug)]
pub enum ExitCode
{
	Quit,
}

#[derive(Debug, Clone)]
pub struct PlatformError
{
	details : String,
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for PlatformError {
    fn description(&self) -> &str {
        &self.details
    }
}

pub type ExitResult = Result<ExitCode, PlatformError>;

pub unsafe extern "system" fn window_proc(
	hwnd : HWND, 
	u_msg : UINT, 
	w_param : WPARAM, 
	l_param : LPARAM) -> LRESULT
{
	match u_msg 
    { 
        WM_CLOSE => {winapi::um::winuser::DestroyWindow(hwnd); 0 }
		WM_DESTROY => {winapi::um::winuser::PostQuitMessage(0); 0 }
        _ => { winapi::um::winuser::DefWindowProcW(hwnd, u_msg, w_param, l_param) },
    }
}

pub fn platform_thread_run(
	window_sender : mpsc::Sender::<win_window::Window>,
	_exit_sender : mpsc::Sender::<ExitResult>,
	_input_sender : mpsc::Sender::<u32>)
{
	let window = win_window::create_window().unwrap();
	win_window::show_window(window);
	window_sender.send(window).expect("Failed to send window out of this thread.");

	loop
	{
		let mut message = mem::MaybeUninit::<MSG>::uninit();
		let mut msg_value = 0;

		// pull off messages until there are no more.
		while unsafe { PeekMessageW(message.as_mut_ptr(), std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0
		{
			msg_value = unsafe { message.assume_init().message };

			// break out and do not process WM_QUIT
			if msg_value == WM_QUIT
			{
				_exit_sender.send(Ok(ExitCode::Quit)).expect("Failed to emit quit message.");
				break;
			}

			unsafe
			{
				TranslateMessage(message.as_ptr());
				DispatchMessageW(message.as_ptr());
			}
		}

		// End Program on WM_QUIT
		if msg_value == WM_QUIT
		{
            break;
        }
    }
}
