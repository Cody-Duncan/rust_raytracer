// Declare Modules
mod win_window;
mod win_utilities;
mod win_platform;
mod dx_renderer;
mod geometry;

// Use Declarations
use std::thread;
use std::sync::mpsc;

// Main Function
fn main() 
{
	println!("Hello, Rust!");

	let (sender, reciever) = mpsc::channel::<win_window::Window>();
	
	let windows_thread= thread::Builder::new()
		.name("win_platform_thread".to_string())
		.spawn(move || {win_platform::platform_thread_run(sender)})
		.expect("failed to spin up win_platform_thread");

	let window = reciever.recv().unwrap();

	let mut renderer = dx_renderer::Renderer::new();
	renderer.load_pipeline(window);
	renderer.load_assets();

	loop
	{
		renderer._update();
		let result = renderer._render();

		if result != 0 
		{ 
			break; 
		}
	}

	windows_thread.join().expect("failed to join win_platform_thread");
}
