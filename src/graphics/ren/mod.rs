pub mod polygons;

//// Helpers ////

pub fn view_matrix(center_x: f32, center_y: f32, scale_x: f32, scale_y: f32) -> [[f32; 4]; 4] {
    // data views the transpose of the actual matrix
    [
        [ scale_x,	0.0, 0.0,	0.0 ],
        [ 	0.0,	scale_y, 0.0,	0.0 ],
        [ 	0.0,	0.0,	1.0,	0.0 ],
        [ -center_x * scale_x,	-center_y * scale_y,	0.0,	1.0]
    ]
}
pub fn proj_matrix(width: f32, height: f32, far: f32, near: f32) -> [[f32; 4]; 4] {
    let width = width as f32;
    let height = height as f32;
    let far = far as f32;
    let near = near as f32;
    [
        [2.0/width, 0.0, 			0.0, 							0.0],
        [0.0, 			 2.0/height,  0.0, 							0.0],
        [0.0, 			 0.0,  			-2.0/(far - near), 			0.0],
        [0.0, 			 0.0, 			-(far + near)/(far - near), 1.0]
    ]
}
