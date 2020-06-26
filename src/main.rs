// Declare Modules
mod win_window;
mod win_utilities;
mod win_platform;

// Use Declarations
use std::thread;

// Main Function
fn main() 
{
	println!("Hello, Rust!");
	
	let windows_thread= thread::Builder::new()
		.name("win_platform_thread".to_string())
		.spawn(win_platform::platform_thread_run)
		.expect("failed to spin up win_platform_thread");

	windows_thread.join().expect("failed to join win_platform_thread");
}
