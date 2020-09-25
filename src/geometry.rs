use cgmath::Vector3;
use cgmath::Vector4;

pub struct ColoredVertex
{
	#[allow(dead_code)]
	position : Vector3<f32>,

	#[allow(dead_code)]
	color : Vector4<f32>
}

static ASPECT_RATIO : f32 = 1280.0/720.0;

pub fn sample_colored_triangle_vertices() -> [ColoredVertex; 3]
{
	return 
	[
		ColoredVertex 
		{ 
			position : Vector3::new(0.0, 0.25 * ASPECT_RATIO, 0.0), 
			color : Vector4::new(1.0, 0.0, 0.0, 1.0)
		},
		ColoredVertex 
		{ 
			position : Vector3::new(0.25, -0.25 * ASPECT_RATIO, 0.0), 
			color : Vector4::new(0.0, 1.0, 0.0, 1.0)
		},
		ColoredVertex 
		{ 
			position : Vector3::new(-0.25, -0.25 * ASPECT_RATIO, 0.0), 
			color : Vector4::new(0.0, 0.0, 1.0, 1.0)
		},
	]
}