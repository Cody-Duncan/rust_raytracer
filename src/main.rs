// Declare Modules
mod win_window;
mod win_utilities;
mod win_platform;

// Use Declarations

// Main Function
fn main() 
{
	println!("Hello, Rust!");
	
	let window = win_window::create_window().unwrap();
	win_window::show_window(window);
	win_platform::message_handle_loop(window);
}
