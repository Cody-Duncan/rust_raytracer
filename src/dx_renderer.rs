extern crate d3d12_rs;
extern crate winapi;
use crate::win_window;
use crate::geometry;
use crate::geometry::*;

use winapi::{
	shared::{
		dxgi, dxgi1_2, dxgi1_3, dxgi1_4, winerror, dxgiformat, dxgitype,  
		minwindef::{FALSE, TRUE, UINT}, 
		basetsd::{SIZE_T},
		ntdef::HANDLE,
	},
	//dxgi1_6, minwindef::TRUE, winerror},
	um::{
		d3d12, 
		d3dcommon,
		d3dcompiler::*,
		d3d12sdklayers,
		synchapi::{CreateEventW, WaitForSingleObject},
		winbase::INFINITE,
	},
	Interface, // uuidof
};

use std::{
	assert,
	ffi::CStr,
	ffi::CString,
	ffi::OsString,
	io::Error,
	mem,
	os::windows::ffi::OsStringExt,
	os::windows::ffi::OsStrExt,
	ptr,
	string::String,
	convert::TryFrom,
};

use d3d12_rs::*;
use d3d12_rs::Blob;
use d3d12_rs::WeakPtr;

static G_SINGLE_NODEMASK : u32 = 0;

#[allow(dead_code)]
pub struct Renderer 
{
	viewport : d3d12::D3D12_VIEWPORT,
	scissor_rect : d3d12::D3D12_RECT,
	factory  : WeakPtr<dxgi1_4::IDXGIFactory4>,
	adapter  : WeakPtr<dxgi1_2::IDXGIAdapter2>,
	device   : WeakPtr<d3d12::ID3D12Device>,
	command_queue : WeakPtr<d3d12::ID3D12CommandQueue>,
	swap_chain : WeakPtr<dxgi1_4::IDXGISwapChain3>,
	rtv_descriptor_heap : WeakPtr<d3d12::ID3D12DescriptorHeap>,
	rtv_descriptor_size : u32,
	command_allocator : WeakPtr::<d3d12::ID3D12CommandAllocator>,
	command_list : WeakPtr::<d3d12::ID3D12GraphicsCommandList>,
	render_targets : [WeakPtr<d3d12::ID3D12Resource>; 3],
	root_signature : WeakPtr<d3d12::ID3D12RootSignature>,
	pipeline_state : WeakPtr<d3d12::ID3D12PipelineState>,
	frame_count : u32,
	frame_index : u32,
	vertex_buffer : WeakPtr<d3d12::ID3D12Resource>,
	vertex_buffer_view : d3d12::D3D12_VERTEX_BUFFER_VIEW,
	fence : WeakPtr<d3d12::ID3D12Fence>,
	fence_value : u64,
	fence_event : HANDLE,
}

#[allow(dead_code)]
fn to_wchar(str : &str) -> Vec<u16> 
{
	std::ffi::OsString::from(str).encode_wide().collect()
}

#[allow(dead_code)]
fn to_cstring(str : &str) -> CString
{
	CString::new(str).unwrap()
}

#[repr(transparent)]
#[allow(non_camel_case_types)]
struct CD3D12_CPU_DESCRIPTOR_HANDLE(winapi::um::d3d12::D3D12_CPU_DESCRIPTOR_HANDLE);

impl CD3D12_CPU_DESCRIPTOR_HANDLE
{
	#[allow(dead_code)]
	pub fn new() -> Self
	{
		Self
		{
			0 : winapi::um::d3d12::D3D12_CPU_DESCRIPTOR_HANDLE { ptr : 0 }
		}
	}

	pub fn offset_cpu_descriptor_handle(
		handle : &winapi::um::d3d12::D3D12_CPU_DESCRIPTOR_HANDLE, 
		offset_index : i32, 
		descriptor_increment_size : u32) -> winapi::um::d3d12::D3D12_CPU_DESCRIPTOR_HANDLE
	{
		let ptr_64 = handle.ptr as i64;
		let offset_index_64 = offset_index as i64;
		let descriptor_increment_size_64 = descriptor_increment_size as i64;
		let result = (ptr_64 + offset_index_64 * descriptor_increment_size_64) as SIZE_T;
		return winapi::um::d3d12::D3D12_CPU_DESCRIPTOR_HANDLE { ptr : result };
	}

	pub fn from_offset(
		other : &winapi::um::d3d12::D3D12_CPU_DESCRIPTOR_HANDLE, 
		offset_index : i32, 
		descriptor_increment_size : u32) -> Self
	{
		Self
		{
			0 : Self::offset_cpu_descriptor_handle(&other, offset_index, descriptor_increment_size)
		}
	}

	pub fn offset(
		&mut self, 
		offset_index : i32, 
		descriptor_increment_size : u32) -> & mut Self
	{
		self.0 = Self::offset_cpu_descriptor_handle(&self.0, offset_index, descriptor_increment_size);
		self
	}
}

impl Renderer 
{

pub fn new() -> Self 
{
	if cfg!(debug_assertions) 
	{
		let mut debug_controller = WeakPtr::<d3d12sdklayers::ID3D12Debug>::null();
        let hr_debug = unsafe {
            winapi::um::d3d12::D3D12GetDebugInterface(&d3d12sdklayers::ID3D12Debug::uuidof(), debug_controller.mut_void())};
		assert!(winerror::SUCCEEDED(hr_debug), "Unable to get D3D12 debug interface. {:x}", hr_debug);
		
		debug_controller.enable_layer();

		unsafe { debug_controller.Release(); } // Clean Up
	}

	Self {
		viewport : d3d12::D3D12_VIEWPORT
		{
			TopLeftX: 0.0,
			TopLeftY: 0.0,
			Width: 1280.0,
			Height: 720.0,
			MinDepth: d3d12::D3D12_MIN_DEPTH,
			MaxDepth: d3d12::D3D12_MAX_DEPTH,
		},
		scissor_rect : d3d12::D3D12_RECT
		{
			left : 0, 
			top : 0,
			right : 1280,
			bottom : 720,
		},
		factory  : WeakPtr::<dxgi1_4::IDXGIFactory4>::null(),
		adapter  : WeakPtr::<dxgi1_2::IDXGIAdapter2>::null(),
		device   : WeakPtr::<d3d12::ID3D12Device>::null(),
		command_queue : WeakPtr::<d3d12::ID3D12CommandQueue>::null(),
		swap_chain : WeakPtr::<dxgi1_4::IDXGISwapChain3>::null(),
		rtv_descriptor_heap : WeakPtr::<d3d12::ID3D12DescriptorHeap>::null(),
		rtv_descriptor_size : 0,
		command_allocator : WeakPtr::<d3d12::ID3D12CommandAllocator>::null(),
		command_list : WeakPtr::<d3d12::ID3D12GraphicsCommandList>::null(),
		render_targets : [WeakPtr::null(); 3],
		root_signature : WeakPtr::<d3d12::ID3D12RootSignature>::null(),
		pipeline_state : WeakPtr::<d3d12::ID3D12PipelineState>::null(),
		frame_count : 2,
		frame_index : 0,
		vertex_buffer : WeakPtr::<d3d12::ID3D12Resource>::null(),
		vertex_buffer_view : unsafe { mem::zeroed() },
		fence : WeakPtr::<d3d12::ID3D12Fence>::null(),
		fence_value : 0,
		fence_event : ptr::null_mut(),
	}
}

pub fn load_pipeline(&mut self, window : win_window::Window)
{
	if cfg!(debug_assertions) 
	{
		let mut debug_controller = WeakPtr::<d3d12sdklayers::ID3D12Debug>::null();
        let hr_debug = unsafe {
            winapi::um::d3d12::D3D12GetDebugInterface(&d3d12sdklayers::ID3D12Debug::uuidof(), debug_controller.mut_void())};
		assert!(winerror::SUCCEEDED(hr_debug), "Unable to get D3D12 debug interface. {:x}", hr_debug);
		
		debug_controller.enable_layer();

		unsafe { debug_controller.Release(); } // Clean Up
	}

	let factory_flags = match cfg!(debug_assertions) {
		true => FactoryCreationFlags::DEBUG,
		false => FactoryCreationFlags::empty()
	};

	let mut factory = WeakPtr::<dxgi1_4::IDXGIFactory4>::null();
	let hr_factory = unsafe {
		dxgi1_3::CreateDXGIFactory2(factory_flags.bits(), &dxgi1_4::IDXGIFactory4::uuidof(), factory.mut_void())};
	assert!(winerror::SUCCEEDED(hr_factory), "Failed on factory creation. {:x}", hr_factory);
	self.factory = factory;

	let mut adapter_index = 0;
	let _adapter = loop
	{
		let mut adapter1 = WeakPtr::<dxgi::IDXGIAdapter1>::null();
		let hr1 = unsafe { self.factory.EnumAdapters1(adapter_index, adapter1.mut_void() as *mut *mut _) };

		if hr1 == winerror::DXGI_ERROR_NOT_FOUND 
		{
			break Err("Failed to enumerate adapters: DXGI_ERROR_NOT_FOUND");
		}

		let (adapter2, hr2) = unsafe { adapter1.cast::<dxgi1_2::IDXGIAdapter2>() };

		unsafe { adapter1.destroy(); } // always clean up

		if !winerror::SUCCEEDED(hr2) 
		{
			break Err("Failed to casting to adapter2.");
		}

		adapter_index += 1;

		// Check to see if the adapter supports Direct3D 12, but don't create the
		// actual device yet.
		let mut _device = WeakPtr::<d3d12::ID3D12Device>::null();
		let hr_device = unsafe {
			d3d12::D3D12CreateDevice(
				adapter2.as_mut_ptr() as *mut _,
				FeatureLevel::L11_0 as _,
				&d3d12::ID3D12Device::uuidof(),
				_device.mut_void(),
			)
		};

		if !winerror::SUCCEEDED(hr_device)
		{
			unsafe { adapter2.destroy(); }; // always clean up before looping back
			continue
		}

		break Ok(adapter2);
	};

	self.adapter = _adapter.expect("Failed to find a reasonable adapter.");

	// create the device for real
	let mut device = WeakPtr::<d3d12::ID3D12Device>::null();
	let hr_device = unsafe {
		d3d12::D3D12CreateDevice(
			self.adapter.as_unknown() as *const _ as *mut _,
			FeatureLevel::L11_0 as _,
			&d3d12::ID3D12Device::uuidof(),
			device.mut_void(),
		)
	};
	assert!(winerror::SUCCEEDED(hr_device), "Failed to create DX12 device. {:x}", hr_device);
	self.device = device;

	// Describe and Create the command queue.
	let desc = d3d12::D3D12_COMMAND_QUEUE_DESC {
		Type: d3d12_rs::CmdListType::Direct as _,
		Priority: d3d12_rs::Priority::Normal as _,
		Flags: d3d12_rs::CommandQueueFlags::empty().bits(),
		NodeMask: G_SINGLE_NODEMASK,
	};

	let mut command_queue = WeakPtr::<d3d12::ID3D12CommandQueue>::null();
	let hr_queue = unsafe {
		self.device.CreateCommandQueue(
			&desc,
			&d3d12::ID3D12CommandQueue::uuidof(),
			command_queue.mut_void(),
		)
	};
	assert!(winerror::SUCCEEDED(hr_queue),"error on queue creation: {:x}", hr_queue);
	self.command_queue = command_queue;
	
	// Create the Swap Chain
	let desc = dxgi1_2::DXGI_SWAP_CHAIN_DESC1 {
		AlphaMode: dxgi1_2::DXGI_ALPHA_MODE_IGNORE,
		BufferCount: self.frame_count,
		Width: 720,
		Height: 1280,
		Format: dxgiformat::DXGI_FORMAT_R8G8B8A8_UNORM,
		Flags: dxgi::DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
		BufferUsage: dxgitype::DXGI_USAGE_RENDER_TARGET_OUTPUT,
		SampleDesc: dxgitype::DXGI_SAMPLE_DESC {
			Count: 1,
			Quality: 0,
		},
		Scaling: dxgi1_2::DXGI_SCALING_STRETCH,
		Stereo: FALSE,
		SwapEffect: dxgi::DXGI_SWAP_EFFECT_FLIP_DISCARD,
	};

	self.swap_chain = unsafe 
	{
		let mut swap_chain1 = d3d12_rs::WeakPtr::<dxgi1_2::IDXGISwapChain1>::null();

		let hr = self.factory.CreateSwapChainForHwnd(
			command_queue.as_mut_ptr() as *mut _,
			window.handle,
			&desc,
			ptr::null(),
			ptr::null_mut(),
			swap_chain1.mut_void() as *mut *mut _,
		);
		assert!(winerror::SUCCEEDED(hr),"error on swapchain creation 0x{:x}", hr);

		let (swap_chain3, hr3) = swap_chain1.cast::<dxgi1_4::IDXGISwapChain3>();
		assert!(winerror::SUCCEEDED(hr), "error on swapchain3 cast 0x{:x}", hr3);

		swap_chain1.destroy();
		swap_chain3
	};

	self.frame_index = unsafe { self.swap_chain.GetCurrentBackBufferIndex() };

	let heap_type = DescriptorHeapType::Rtv;

	// Create Descriptor Heaps
	let mut rtv_descriptor_heap = WeakPtr::<d3d12::ID3D12DescriptorHeap>::null();
	let descriptor_heap_desc = d3d12::D3D12_DESCRIPTOR_HEAP_DESC 
	{
		Type: heap_type as _,
		NumDescriptors: self.frame_count,
		Flags: DescriptorHeapFlags::empty().bits(),
		NodeMask: G_SINGLE_NODEMASK,
	};
	let descriptor_heap_hr = unsafe {
		self.device.CreateDescriptorHeap(
			&descriptor_heap_desc,
			&d3d12::ID3D12DescriptorHeap::uuidof(),
			rtv_descriptor_heap.mut_void())
		};
	assert!(winerror::SUCCEEDED(descriptor_heap_hr), "error on descriptor_heap creation 0x{:x}", descriptor_heap_hr);
	self.rtv_descriptor_heap = rtv_descriptor_heap;

	let rtv_descriptor_size = self.device.get_descriptor_increment_size(heap_type);
	self.rtv_descriptor_size = rtv_descriptor_size;
	let rtv_heap_cpu_handle = rtv_descriptor_heap.start_cpu_descriptor();
	let _rtv_heap_gpu_handle = rtv_descriptor_heap.start_gpu_descriptor();

	let write_render_targets= & mut self.render_targets[0..(self.frame_count as usize)];

	let mut rtv_cpu_handle = CD3D12_CPU_DESCRIPTOR_HANDLE::from_offset(&rtv_heap_cpu_handle, 0, self.rtv_descriptor_size);
	for n in 0..write_render_targets.len()
	{
		unsafe
		{
			let render_target_ref = & mut write_render_targets[n];
			self.swap_chain.GetBuffer(n as _, &d3d12::ID3D12Resource::uuidof(), render_target_ref.mut_void());
			self.device.CreateRenderTargetView(render_target_ref.as_mut_ptr(), ptr::null(), rtv_cpu_handle.0);
			rtv_cpu_handle.offset(1, rtv_descriptor_size);
		}
	}

	// Create Command Allocator
	let (command_allocator, command_allocator_hr) = self.device.create_command_allocator(CmdListType::Direct);
	assert!(winerror::SUCCEEDED(command_allocator_hr), "Failed to create command allocator. 0x{:x}", command_allocator_hr);
	self.command_allocator = command_allocator;
}

pub fn _get_adapter_name(adapter: d3d12_rs::WeakPtr<dxgi1_2::IDXGIAdapter2>) -> String
{
	let mut desc: dxgi1_2::DXGI_ADAPTER_DESC2 = unsafe { mem::zeroed() };
	unsafe { adapter.GetDesc2(&mut desc); }

	let device_name = {
		let len = desc.Description.iter().take_while(
			|&&c| c != 0) // closure: func(&&c) { return c != 0; }
			.count();
		let name = <OsString as OsStringExt>::from_wide(&desc.Description[..len]);
		name.to_string_lossy().into_owned()
	};

	// Handy to know these are available.
	//let _name = _device_name;
	//let _vendor = desc.VendorId as usize;
	//let _device = desc.DeviceId as usize;

	return device_name;
}

pub fn _get_additional_device_data(device: d3d12_rs::WeakPtr<d3d12::ID3D12Device>)
{
	let mut features_architecture: d3d12::D3D12_FEATURE_DATA_ARCHITECTURE = unsafe { mem::zeroed() };
	assert_eq!(winerror::S_OK, 
		unsafe 
		{
			device.CheckFeatureSupport(
				d3d12::D3D12_FEATURE_ARCHITECTURE,
				&mut features_architecture as *mut _ as *mut _, // take reference, cast to pointer, cast to void pointer
				mem::size_of::<d3d12::D3D12_FEATURE_DATA_ARCHITECTURE>() as _)
		});
	
	let mut features: d3d12::D3D12_FEATURE_DATA_D3D12_OPTIONS = unsafe { mem::zeroed() };
	assert_eq!(winerror::S_OK, 
		unsafe 
		{
			device.CheckFeatureSupport(
				d3d12::D3D12_FEATURE_D3D12_OPTIONS,
				&mut features as *mut _ as *mut _,
				mem::size_of::<d3d12::D3D12_FEATURE_DATA_D3D12_OPTIONS>() as _)
		});
}

pub fn load_assets(&mut self)
{
	// Create an empty Root Signature
	let mut signature_raw = WeakPtr::<d3dcommon::ID3DBlob>::null();
	let mut signature_error = WeakPtr::<d3dcommon::ID3DBlob>::null();
	let parameters: &[RootParameter] = &[];
	let static_samplers: &[StaticSampler] = &[];
	let flags = d3d12_rs::RootSignatureFlags::ALLOW_IA_INPUT_LAYOUT;
	
	let root_signature_desc = d3d12::D3D12_ROOT_SIGNATURE_DESC {
		NumParameters: parameters.len() as _,
		pParameters: parameters.as_ptr() as *const _,
		NumStaticSamplers: static_samplers.len() as _,
		pStaticSamplers: static_samplers.as_ptr() as _,
		Flags: flags.bits(),
	};

	let hr_seralize_root_signature = unsafe {
		d3d12::D3D12SerializeRootSignature(
			&root_signature_desc,
			d3d12_rs::RootSignatureVersion::V1_0 as _,
			signature_raw.mut_void() as *mut *mut _,
			signature_error.mut_void() as *mut *mut _,
		)
	};
	assert!(winerror::SUCCEEDED(hr_seralize_root_signature), "Failed to serialize root signature. 0x{:x}", hr_seralize_root_signature);

	if !signature_error.is_null() 
	{
		println!(
			"Root signature serialization error: {:?}",
			unsafe { signature_error.as_c_str().to_str().unwrap() }
		);
		unsafe { signature_error.destroy(); }
	}

	// Create the pipline state, which includes compiling and loading shaders.
	let mut root_signature = RootSignature::null();
	let root_signature_hr = unsafe {
		self.device.CreateRootSignature(
			G_SINGLE_NODEMASK,
			signature_raw.GetBufferPointer(),
			signature_raw.GetBufferSize(),
			&d3d12::ID3D12RootSignature::uuidof(),
			root_signature.mut_void(),
		)};
	assert!(winerror::SUCCEEDED(root_signature_hr), "Failed to create root signature. 0x{:x}", root_signature_hr);
    unsafe { signature_raw.destroy(); } 

	self.root_signature = root_signature;

	let compile_flags = if cfg!(debug_assertions) { D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION } else { 0 };

	let shader_path = to_wchar("D:\\Repo\\rust\\rust_raytracer\\src\\shaders.hlsl");

	let vertex_shader_entry_point = to_cstring("VSMain");
	let vertex_shader_compiler_target = to_cstring("vs_5_0");

	let pixel_shader_entry_point = to_cstring("PSMain");
	let pixel_shader_compiler_target = to_cstring("ps_5_0");

	let mut vertex_shader_blob : WeakPtr<d3dcommon::ID3DBlob> = Blob::null();
	let mut vertex_shader_error : WeakPtr<d3dcommon::ID3DBlob> = Blob::null();
	let mut pixel_shader_blob : WeakPtr<d3dcommon::ID3DBlob> = Blob::null();
	let mut pixel_shader_error : WeakPtr<d3dcommon::ID3DBlob> = Blob::null();

	unsafe 
	{
		let hr_vertex_shader_compile = D3DCompileFromFile(
			shader_path.as_ptr(),
			ptr::null() as _,
			ptr::null_mut() as _,
			vertex_shader_entry_point.as_ptr(),
			vertex_shader_compiler_target.as_ptr(),
			compile_flags,
			0,
			vertex_shader_blob.mut_void() as *mut *mut d3dcommon::ID3DBlob,
			vertex_shader_error.mut_void() as *mut *mut d3dcommon::ID3DBlob);

		if !winerror::SUCCEEDED(hr_vertex_shader_compile)
		{
			let error_result = CString::from(CStr::from_ptr(vertex_shader_error.GetBufferPointer() as * const i8));
			let error_result_str = error_result.to_str().unwrap();
			
			assert!(winerror::SUCCEEDED(hr_vertex_shader_compile), "Failed to compile vertex shader. HRESULT: 0x{0:x} ; path: {1} ; Error {2} ; Shader Blob Error {3}", 
				hr_vertex_shader_compile, 
				String::from_utf16(&shader_path).unwrap(),
				std::io::Error::from_raw_os_error(hr_vertex_shader_compile),
				error_result_str);
		}
			
		assert!(!vertex_shader_blob.is_null(), "Failed to create vertex shader. path: {0}", String::from_utf16(&shader_path).unwrap());

		let hr_pixel_shader_compile = D3DCompileFromFile(
			shader_path.as_ptr(),
			ptr::null() as _,
			ptr::null_mut() as _,
			pixel_shader_entry_point.as_ptr(),
			pixel_shader_compiler_target.as_ptr(),
			compile_flags,
			0,
			pixel_shader_blob.mut_void() as *mut *mut d3dcommon::ID3DBlob,
			pixel_shader_error.mut_void() as *mut *mut d3dcommon::ID3DBlob);

		if !winerror::SUCCEEDED(hr_pixel_shader_compile)
		{
			let error_result = CString::from(CStr::from_ptr(pixel_shader_error.GetBufferPointer() as * const i8));
			let error_result_str = error_result.to_str().unwrap();
			
			assert!(winerror::SUCCEEDED(hr_pixel_shader_compile), "Failed to compile pixel shader. HRESULT: 0x{0:x} ; path: {1} ; Error {2} ; Shader Blob Error {3}", 
				hr_pixel_shader_compile, 
				String::from_utf16(&shader_path).unwrap(),
				std::io::Error::from_raw_os_error(hr_pixel_shader_compile),
				error_result_str);
		}

		assert!(!pixel_shader_blob.is_null(), "Failed to create pixel shader. path: {0}", String::from_utf16(&shader_path).unwrap());
	}

	let position_semantic = to_cstring("POSITION");
	let color_semnatic = to_cstring("COLOR");

	let input_element_descs : [d3d12::D3D12_INPUT_ELEMENT_DESC; 2] =
	[
		d3d12::D3D12_INPUT_ELEMENT_DESC { 
			SemanticName: position_semantic.as_ptr(),
			SemanticIndex: 0,
			Format: dxgiformat::DXGI_FORMAT_R32G32B32_FLOAT,
			InputSlot: 0,
			AlignedByteOffset: 0,
			InputSlotClass: d3d12::D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
			InstanceDataStepRate: 0, },
		d3d12::D3D12_INPUT_ELEMENT_DESC { 
			SemanticName: color_semnatic.as_ptr(),
			SemanticIndex: 0,
			Format: dxgiformat::DXGI_FORMAT_R32G32B32_FLOAT,
			InputSlot: 0,
			AlignedByteOffset: 12,
			InputSlotClass: d3d12::D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
			InstanceDataStepRate: 0,}
	];

	let vertex_shader = d3d12_rs::Shader::from_blob(vertex_shader_blob);
	let pixel_shader = d3d12_rs::Shader::from_blob(pixel_shader_blob);

	let default_render_target_blend_desc=
		d3d12::D3D12_RENDER_TARGET_BLEND_DESC
        {
			BlendEnable : FALSE,
			LogicOpEnable : FALSE,
			SrcBlend : d3d12::D3D12_BLEND_ONE,
			DestBlend : d3d12::D3D12_BLEND_ZERO,
			BlendOp : d3d12::D3D12_BLEND_OP_ADD,
			SrcBlendAlpha : d3d12::D3D12_BLEND_ONE,
			DestBlendAlpha : d3d12::D3D12_BLEND_ZERO,
			BlendOpAlpha : d3d12::D3D12_BLEND_OP_ADD,
            LogicOp: d3d12::D3D12_LOGIC_OP_NOOP,
            RenderTargetWriteMask : d3d12::D3D12_COLOR_WRITE_ENABLE_ALL as u8,
        };

	let default_blendstate=
		d3d12::D3D12_BLEND_DESC 
		{
			AlphaToCoverageEnable: FALSE,
			IndependentBlendEnable: TRUE,
			RenderTarget: [default_render_target_blend_desc; 8],
		};

	let default_rasterizer_state =
		d3d12::D3D12_RASTERIZER_DESC
		{
			FillMode : d3d12::D3D12_FILL_MODE_SOLID,
			CullMode : d3d12::D3D12_CULL_MODE_BACK,
			FrontCounterClockwise : FALSE,
			DepthBias : d3d12::D3D12_DEFAULT_DEPTH_BIAS as i32,
			DepthBiasClamp : d3d12::D3D12_DEFAULT_DEPTH_BIAS_CLAMP,
			SlopeScaledDepthBias : d3d12::D3D12_DEFAULT_SLOPE_SCALED_DEPTH_BIAS,
			DepthClipEnable : TRUE,
			MultisampleEnable : FALSE,
			AntialiasedLineEnable : FALSE,
			ForcedSampleCount : 0,
			ConservativeRaster : d3d12::D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
		};

	let default_depth_stencil_op_desc = 
		d3d12::D3D12_DEPTH_STENCILOP_DESC 
		{
			StencilFailOp: 0,
			StencilDepthFailOp: 0,
			StencilPassOp: 0,
			StencilFunc: 0,
		};

	let depth_stencil_state_desc = d3d12::D3D12_DEPTH_STENCIL_DESC
	{
		DepthEnable: FALSE,
		DepthWriteMask: d3d12::D3D12_DEPTH_WRITE_MASK_ZERO,
		DepthFunc: d3d12::D3D12_COMPARISON_FUNC_NEVER,
		StencilEnable: FALSE,
		StencilReadMask: 0,
		StencilWriteMask: 0,
		FrontFace: default_depth_stencil_op_desc,
		BackFace: default_depth_stencil_op_desc,
	};

	let mut default_rtv_formats = [dxgiformat::DXGI_FORMAT_UNKNOWN;8];
	default_rtv_formats[0] = dxgiformat::DXGI_FORMAT_R8G8B8A8_UNORM;

	 // Setup pipeline description
	 let pso_desc = d3d12::D3D12_GRAPHICS_PIPELINE_STATE_DESC 
	 {
		pRootSignature: self.root_signature.as_mut_ptr(),
		VS: *vertex_shader,
		PS: *pixel_shader,
		GS: *d3d12_rs::Shader::null(),
		DS: *d3d12_rs::Shader::null(),
		HS: *d3d12_rs::Shader::null(),
		StreamOutput : d3d12::D3D12_STREAM_OUTPUT_DESC 
		{
			pSODeclaration: ptr::null(),
			NumEntries: 0,
			pBufferStrides: ptr::null(),
			NumStrides: 0,
			RasterizedStream: 0,
		},
		BlendState: default_blendstate,
		SampleMask: UINT::max_value(),
		RasterizerState: default_rasterizer_state,
		DepthStencilState: depth_stencil_state_desc,
		InputLayout: d3d12::D3D12_INPUT_LAYOUT_DESC {
			pInputElementDescs: input_element_descs.as_ptr(),
			NumElements: input_element_descs.len() as u32,
		},
		IBStripCutValue: d3d12::D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED,
		PrimitiveTopologyType: d3d12::D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
		NumRenderTargets: self.frame_count,
		RTVFormats: default_rtv_formats,
		DSVFormat: dxgiformat::DXGI_FORMAT_UNKNOWN,
		SampleDesc: dxgitype::DXGI_SAMPLE_DESC { Count: 1, Quality: 0},
		NodeMask: 0,
		CachedPSO: d3d12::D3D12_CACHED_PIPELINE_STATE {
			pCachedBlob: ptr::null(),
			CachedBlobSizeInBytes: 0,
			},
		Flags: d3d12::D3D12_PIPELINE_STATE_FLAG_NONE,
	};

	// Create Pipeline State
	let mut pipeline = d3d12_rs::PipelineState::null();
	unsafe 
	{
		let hr_gpstate = self.device.CreateGraphicsPipelineState(
			&pso_desc,
			&d3d12::ID3D12PipelineState::uuidof(),
			pipeline.mut_void());

		assert!(winerror::SUCCEEDED(hr_gpstate), "Failed to create graphics pipeline state. 0x{:x}", hr_gpstate);
	}
	self.pipeline_state = pipeline;

	// Create the Command List
	let mut command_list = WeakPtr::<d3d12::ID3D12GraphicsCommandList>::null();
	unsafe 
	{
		let hr_create_command_list = self.device.CreateCommandList(
			G_SINGLE_NODEMASK,
			d3d12::D3D12_COMMAND_LIST_TYPE_DIRECT,
			self.command_allocator.as_mut_ptr(),
			pipeline.as_mut_ptr(),
			&d3d12::ID3D12GraphicsCommandList::uuidof(),
			command_list.mut_void());

		assert!(winerror::SUCCEEDED(hr_create_command_list), "Failed to create command list. 0x{:x}", hr_create_command_list);
		
		// Command lists are created in the recording state, but there is nothing
    	// to record yet. The main loop expects it to be closed, so close it now.
		command_list.Close();
	}
	self.command_list = command_list;

	// Create Triangle Assets
	// Upload to Vertex Buffer.
	{
		let mut triangle_vertices : [geometry::ColoredVertex ; 3] = sample_colored_triangle_vertices();
		let triangle_vertices_size = std::mem::size_of_val(&triangle_vertices);
		let triangle_vertices_size_u32 = u32::try_from(triangle_vertices_size).expect("Failed Type Conversion: usize -> u32");
		let vertex_size = std::mem::size_of_val(&triangle_vertices[0]);
		let vertex_size_u32 = u32::try_from(vertex_size).expect("Failed Type Conversion: usize -> u32");

		assert!(triangle_vertices_size == 84);

		let default_heap_properties = d3d12::D3D12_HEAP_PROPERTIES {
			Type: d3d12::D3D12_HEAP_TYPE_UPLOAD,
			CPUPageProperty: d3d12::D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
			MemoryPoolPreference: d3d12::D3D12_MEMORY_POOL_UNKNOWN,
			CreationNodeMask: G_SINGLE_NODEMASK,
			VisibleNodeMask: G_SINGLE_NODEMASK,
		};

		let vertex_buffer_resource_desc = d3d12::D3D12_RESOURCE_DESC {
			Dimension: d3d12::D3D12_RESOURCE_DIMENSION_BUFFER,
			Alignment: 0,
			Width: triangle_vertices_size as u64,
			Height: 1,
			DepthOrArraySize: 1,
			MipLevels: 1,
			Format: dxgiformat::DXGI_FORMAT_UNKNOWN,
			SampleDesc: dxgitype::DXGI_SAMPLE_DESC {
				Count: 1,
				Quality: 0,
			},
			Layout: d3d12::D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
			Flags: d3d12::D3D12_RESOURCE_FLAG_NONE,
		};

		let mut vertex_buffer = WeakPtr::<d3d12::ID3D12Resource>::null();

		unsafe 
		{		
			let hr_create_committed_resource = self.device.CreateCommittedResource(
				&default_heap_properties,
				d3d12::D3D12_HEAP_FLAG_NONE,
				&vertex_buffer_resource_desc,
				d3d12::D3D12_RESOURCE_STATE_GENERIC_READ,
				ptr::null() as _,
				&d3d12::ID3D12Resource::uuidof(),
				vertex_buffer.mut_void());

			assert!(winerror::SUCCEEDED(hr_create_committed_resource), "Failed to create vertex buffer. 0x{:x}", hr_create_committed_resource);

			let buffer_name : String = String::from("triangle vertex buffer");
			let buffer_size = u32::try_from(buffer_name.len()).unwrap();
			vertex_buffer.SetPrivateData(&d3dcommon::WKPDID_D3DDebugObjectName, buffer_size, buffer_name.as_ptr() as * mut _);
		}
		
		let mut p_vertex_data_begin = ptr::null_mut::<winapi::ctypes::c_void>();

		let read_range = d3d12::D3D12_RANGE { Begin: 0 , End: 0};
		unsafe 
		{
			let hr_map = vertex_buffer.Map(0, &read_range, &mut p_vertex_data_begin);
			assert!(winerror::SUCCEEDED(hr_map), "Failed to map vertex buffer. 0x{:x}", hr_map);
			assert!(!p_vertex_data_begin.is_null(), "Failed to map vertex buffer. 0x{:x}", hr_map);

			std::ptr::copy_nonoverlapping(
				triangle_vertices.as_mut_ptr(),
				p_vertex_data_begin as * mut ColoredVertex,
				triangle_vertices_size);

			vertex_buffer.Unmap(0, ptr::null());
		}
		
		self.vertex_buffer = vertex_buffer;
		self.vertex_buffer_view = d3d12::D3D12_VERTEX_BUFFER_VIEW {
			BufferLocation: unsafe { self.vertex_buffer.GetGPUVirtualAddress() },
			SizeInBytes: triangle_vertices_size_u32,
			StrideInBytes: vertex_size_u32,
		};
		assert!(self.vertex_buffer_view.StrideInBytes == 28);
		assert!(self.vertex_buffer_view.SizeInBytes == 84);
	}

	// Create synchronization objects and wait until assets have been uploaded to the GPU.
	{
		unsafe 
		{
			let initial_value_zero = 0;
			let hr_create_fence = self.device.CreateFence(
				initial_value_zero,
				d3d12::D3D12_FENCE_FLAG_NONE,
				&d3d12::ID3D12Fence::uuidof(),
				self.fence.mut_void());
			assert!(winerror::SUCCEEDED(hr_create_fence), "Failed to create fence. 0x{:x}", hr_create_fence);
			self.fence_value = 1;

			// Create an event handle to use for frame synchronization.
			self.fence_event = CreateEventW(ptr::null_mut(), FALSE, FALSE, ptr::null());
			assert!(self.fence_event != ptr::null_mut(), "Failed to create fence. 0x{:?}", Error::last_os_error());

			// Wait for the command list to execute; we are reusing the same command 
			// list in our main loop but for now, we just want to wait for setup to 
			// complete before continuing.
			self.wait_for_previous_frame();
		}
	}
}

pub fn _update(&mut self)
{
	// cool. Nothing to do here.
}

pub fn _render(&mut self) -> i32
{
	self._populate_command_list();

	unsafe 
	{
		let vec_command_lists = [self.command_list.as_mut_ptr() as * mut d3d12::ID3D12CommandList];
		self.command_queue.ExecuteCommandLists(u32::try_from(vec_command_lists.len()).unwrap(), vec_command_lists.as_ptr());

		let hr_swap_backbuffer = self.swap_chain.Present(1, 0);
		assert!(winerror::SUCCEEDED(hr_swap_backbuffer), "Failed to swap backbuffer. 0x{:x}", hr_swap_backbuffer);
	}

	self.wait_for_previous_frame();

	return 0;
}

pub fn _destroy(&mut self)
{
	// Ensure that the GPU is no longer referencing resources that are about to be
    // cleaned up by the destructor.
	self.wait_for_previous_frame();

    unsafe { winapi::um::handleapi::CloseHandle(self.fence_event); }
}

pub fn _populate_command_list(&mut self)
{
	unsafe 
	{ 
		let hr_allocator_reset = self.command_allocator.Reset();
		assert!(winerror::SUCCEEDED(hr_allocator_reset), "Failed to reset command allocator. 0x{:x}", hr_allocator_reset);

		let hr_command_reset = self.command_list.Reset(self.command_allocator.as_mut_ptr(), self.pipeline_state.as_mut_ptr());
		assert!(winerror::SUCCEEDED(hr_command_reset), "Failed to reset command list. 0x{:x}", hr_command_reset);

		self.command_list.SetGraphicsRootSignature(self.root_signature.as_mut_ptr());
		self.command_list.RSSetViewports(1, &self.viewport);
		self.command_list.RSSetScissorRects(1, &self.scissor_rect);

		let resource_barrier_start = d3d12_rs::ResourceBarrier::transition(
			self.render_targets[self.frame_index as usize], 
			d3d12::D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, 
			d3d12::D3D12_RESOURCE_STATE_PRESENT, 
			d3d12::D3D12_RESOURCE_STATE_RENDER_TARGET, 
			d3d12::D3D12_RESOURCE_BARRIER_FLAG_NONE);
		let resource_barrier_start_d3d = std::mem::transmute::<& d3d12_rs::ResourceBarrier, * const d3d12::D3D12_RESOURCE_BARRIER>(&resource_barrier_start);
		self.command_list.ResourceBarrier(1, resource_barrier_start_d3d);

		let rtv_handle = CD3D12_CPU_DESCRIPTOR_HANDLE::from_offset(&self.rtv_descriptor_heap.GetCPUDescriptorHandleForHeapStart(), self.frame_index as i32, self.rtv_descriptor_size);
		self.command_list.OMSetRenderTargets(1, &rtv_handle.0, FALSE, ptr::null());

		let clear_color : [f32 ; 4] = [0.0, 0.2, 0.4, 1.0];
		self.command_list.ClearRenderTargetView(rtv_handle.0, &clear_color, 0, ptr::null());

		self.command_list.IASetPrimitiveTopology(d3dcommon::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
		self.command_list.IASetVertexBuffers(0, 1, &self.vertex_buffer_view);
		let vertex_count = 3;
		let instance_count= 1;
		let start_vertex_location = 0;
		let start_instance_location = 0;
		self.command_list.DrawInstanced(vertex_count, instance_count, start_vertex_location, start_instance_location);

		let resource_barrier_end = d3d12_rs::ResourceBarrier::transition(
			self.render_targets[self.frame_index as usize], 
			d3d12::D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, 
			d3d12::D3D12_RESOURCE_STATE_RENDER_TARGET, 
			d3d12::D3D12_RESOURCE_STATE_PRESENT, 
			d3d12::D3D12_RESOURCE_BARRIER_FLAG_NONE);
		let resource_barrier_end_d3d = std::mem::transmute::<& d3d12_rs::ResourceBarrier, * const d3d12::D3D12_RESOURCE_BARRIER>(&resource_barrier_end);
		self.command_list.ResourceBarrier(1, resource_barrier_end_d3d);

		let hr_command_close = self.command_list.Close();
		assert!(winerror::SUCCEEDED(hr_command_close), "Failed to close command list. 0x{:x}", hr_command_close);
	}
}

pub fn wait_for_previous_frame(&mut self)
{
	// WAITING FOR THE FRAME TO COMPLETE BEFORE CONTINUING IS NOT BEST PRACTICE.
	// This is code implemented as such for simplicity.
	
	// Signal and increment the fence value.
	let current_fence_value = self.fence_value;
	unsafe 
	{
		let hr_signal = self.command_queue.Signal(self.fence.as_mut_ptr(), current_fence_value);
		assert!(winerror::SUCCEEDED(hr_signal), "Failed to signal the comment queue. 0x{:x}", hr_signal);
		// The fence is now set into the command queue, which will update the fence with the current value.
	}
	self.fence_value += 1;

	// Wait until the previous frame is finished.
	if (unsafe {self.fence.GetCompletedValue()} < current_fence_value)
	{
		unsafe 
		{
			// Fire the event once the fence has been updated to the current value.
			let hr_on_completed = self.fence.SetEventOnCompletion(current_fence_value, self.fence_event);
			assert!(winerror::SUCCEEDED(hr_on_completed), "Failed to SetEventOnCompletion. 0x{:x}", hr_on_completed);

			// Wait for the fence event (end of command queue)
			let wait_result = WaitForSingleObject(self.fence_event, INFINITE);

			match wait_result 
			{
				0x00000080 => println!("wait_for_previous_frame: WAIT_ABANDONED"),
				0x00000000 => (), // println!("wait_for_previous_frame: WAIT_OBJECT_0"),
				0x00000102 => println!("wait_for_previous_frame: WAIT_TIMEOUT"),
				0xFFFFFFFF => {println!("wait_for_previous_frame: WAIT_FAILED") ; panic!("wait_for_previous_frame failed") },
				_ => unreachable!(),
			}
		}
	}

	// Swap backbuffer index for the new frame
	unsafe 
	{
		self.frame_index = self.swap_chain.GetCurrentBackBufferIndex();
	}
}

}