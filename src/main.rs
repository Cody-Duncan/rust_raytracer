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

	let (window_sender, window_reciever) = mpsc::channel::<win_window::Window>();
	let (exit_sender, _exit_receiver) = mpsc::channel::<win_platform::ExitResult>();
	let (input_sender, _input_receiver) = mpsc::channel::<u32>();
	
	let windows_thread= thread::Builder::new()
		.name("win_platform_thread".to_string())
		.spawn(move || {win_platform::platform_thread_run(window_sender, exit_sender, input_sender)})
		.expect("failed to spin up win_platform_thread");

	let window = window_reciever.recv().unwrap();

	let mut renderer = dx_renderer::Renderer::new();
	renderer.load_pipeline(window);
	renderer.load_assets();

	use std::time::{Instant};

	let now = Instant::now();
	let mut second_fence = 0;
	let mut count = 0;
	
	println!("{}", now.elapsed().as_secs());

	loop
	{
		let current_seconds = now.elapsed().as_secs();
		if current_seconds > second_fence
		{
			second_fence = current_seconds;
			let fps = count / current_seconds;
			println!("FPS: {:?}", fps);
		}
		
		renderer.update();
		let result = renderer.render();

		if result != 0
		{ 
			break; 
		}

		let try_exit_result = _exit_receiver.try_recv();
		match try_exit_result
		{
			Err(_) => (), // nothing received fromt the platform thread
			Ok(exit_result) => 
			{
				match exit_result
				{
					Ok(exit_code) => println!("Platform Thread Exited Successfully. Exit Code {:?}", exit_code),
					Err(platform_error) => println!("Platform Thread Exited with Error: {:?}", platform_error),
				}
				break
			}
		}

		count += 1;
		
	}

	windows_thread.join().expect("failed to join win_platform_thread");
}
