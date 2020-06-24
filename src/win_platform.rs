use std::mem;

use winapi::um::winuser::{
	MSG,
	PM_REMOVE,
	WM_QUIT,
    TranslateMessage,
    DispatchMessageW,
	PeekMessageW,
};

use crate::win_window::Window;

pub fn handle_message( window : Window ) -> bool 
{
	let mut message = mem::MaybeUninit::<MSG>::uninit();

	let _h_res_get_message = 
		unsafe { PeekMessageW(message.as_mut_ptr(), window.handle, 0, 0, PM_REMOVE) };

	let msg_value : u32 = unsafe { message.assume_init().message };

	if msg_value != WM_QUIT // WM_QUIT
	{
		unsafe
		{
			TranslateMessage(message.as_ptr());
			DispatchMessageW(message.as_ptr());
		}
		return true;
	}
	else 
	{
		return false;
	}
}

pub fn message_handle_loop( window : Window )
{
	loop 
	{
		if !handle_message( window ) 
		{
            break;
        }
    }
}