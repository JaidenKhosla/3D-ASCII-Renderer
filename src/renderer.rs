use std::cell::RefMut;
use std::{cell::RefCell, rc::Rc, io::Write};

use crate::{BUFFER_SIZE, HEADER_SIZE, T_COLS, T_ROWS, CAMERA_POS,};
use crate::{Color, Mesh, Vec2, Vec3};

pub struct Renderer
{
    character_buffer: [char; BUFFER_SIZE],
    color_buffer: [Color; BUFFER_SIZE],
    depth_buffer: [f32; BUFFER_SIZE],
    meshes: Vec<Rc<RefCell<Mesh>>>
}

impl Renderer
{
    pub fn new() -> Self
    {
        let mut character_buffer = [' '; BUFFER_SIZE];
        let color_buffer = [Color::Reset; BUFFER_SIZE];
        let depth_buffer = [0.0; BUFFER_SIZE];

        character_buffer[0] = '\x1b';
        character_buffer[1] = '[';
        character_buffer[2] = 'H';

        Renderer { character_buffer, color_buffer, depth_buffer: depth_buffer, meshes: Vec::new() }
    }

    pub fn get_mesh(&self, idx: usize) -> RefMut<Mesh>
    {
        self.meshes[idx].borrow_mut()
    }

    fn draw_mesh(&mut self, mesh: &Mesh)
    {   
        for triangle_idx in 0..mesh.triangles.len()
        {   
            let triangle = mesh.triangles[triangle_idx];
            
            let sym = mesh.symbols[triangle_idx];
            let color = mesh.colors[triangle_idx];

            let modified_triangle = triangle.map(|idx| mesh.vertices[idx].rotate_x(mesh.rotation[0]).rotate_y(mesh.rotation[1]).rotate_z(mesh.rotation[2]).translate(mesh.translation[0], mesh.translation[1], mesh.translation[2] + 8.0).scale(72.0*mesh.scale[0], 24.0*mesh.scale[1], 1.0*mesh.scale[2]));
            
            let [projected_vec0, projected_vec1, projected_vec2] = modified_triangle.map(|vec_i| vec_i.project().extend(vec_i.z));
            
            let d0 = Vec3 { x: modified_triangle[1].x - modified_triangle[0].x, y: modified_triangle[1].y - modified_triangle[0].y, z: modified_triangle[1].z - modified_triangle[0].z};
            let d1 = Vec3 { x: modified_triangle[2].x - modified_triangle[0].x, y: modified_triangle[2].y - modified_triangle[0].y, z: modified_triangle[2].z - modified_triangle[0].z};
            
            let relative_camera_pos = Vec3 { x: CAMERA_POS.x - modified_triangle[0].x, y: CAMERA_POS.y - modified_triangle[0].y, z: CAMERA_POS.z - modified_triangle[0].z};

            let cross = Vec3::cross_product(&d0, &d1);
            let camera_dot = Vec3::dot_product(&cross, &relative_camera_pos);

            // back face culling
            if camera_dot > 0.0
            {
                continue;
            }



            self.draw_triangle(&projected_vec0, &projected_vec1, &projected_vec2, sym, color);
        }
    }

    fn draw_line(&mut self, x1: usize, x2: usize, y: usize, z: f32, dz_dx: f32, sym: char, color: Color)
    {
        let min = x1.min(x2);
        let max = x1.max(x2);

        for x in min..=max
        {
            let dx = x - min;

            let new_z = z + dz_dx * (dx as f32);

            self.write_to_buffer(y, x, new_z, sym, color);
        }
    }


    fn draw_flat_bottom_triangle(&mut self, top: &Vec3, b1: &Vec3, b2: &Vec3, sym: char, color: Color)
    {
        let slope1 = (top.x - b1.x)/(top.y - b1.y);
        let slope2 = (top.x - b2.x)/(top.y - b2.y);

        let delta_y = (b1.y - top.y).round() as usize;

        let dz_dx = (b1.z - b2.z)/(b1.x - b2.x);
        let dz_dy = (top.z - b2.z)/(top.y - b2.y);

        let mut x1 = top.x;
        let mut x2 = top.x;

        let mut z = top.z; 


        for y in 0..=delta_y
        {
            let starting_z = z + (x1.min(x2) - x2) * dz_dx;

            self.draw_line(x1.round() as usize, x2.round() as usize, top.y.round() as usize + y, starting_z, dz_dx, sym, color);

            z += dz_dy;
            x1 += slope1;
            x2 += slope2;
        }
    }

    fn draw_flat_top_triangle(&mut self, bottom: &Vec3, t1: &Vec3, t2: &Vec3, sym: char, color: Color)
    {  
        let slope1 = (bottom.x - t1.x)/(bottom.y - t1.y);
        let slope2 = (bottom.x - t2.x)/(bottom.y - t2.y);

        let delta_y = (bottom.y - t1.y).round() as usize;
        
        let dz_dx = (t1.z - t2.z)/(t1.x - t2.x);
        let dz_dy = (bottom.z - t2.z)/(bottom.y - t2.y);

        let mut x1 = bottom.x;
        let mut x2 = bottom.x;

        let mut z = bottom.z;

        for y in 0..=delta_y
        {
            let starting_z = z + (x1.min(x2) - x2) * dz_dx;

            let y_level = bottom.y.round() as i32 - y as i32;

            if y_level >= 0
            {
                self.draw_line(x1.round() as usize, x2.round() as usize, y_level as usize, starting_z, dz_dx, sym, color);
            }

            z -= dz_dy;
            x1 -= slope1;
            x2 -= slope2;

        }
    }

    fn draw_triangle(&mut self, a: &Vec3, b: &Vec3, c: &Vec3, sym: char, color: Color)
    {
        let mut vertices = [a,b,c];

        vertices.sort_by(|a, b| a.y.total_cmp(&b.y));

        let [vec_a, vec_b, vec_c] = vertices;

        if vec_a.y == vec_b.y
        {
            self.draw_flat_top_triangle(vec_c, vec_a, vec_b, sym, color);
            return;
        }

        if vec_b.y == vec_c.y
        {
            self.draw_flat_bottom_triangle(vec_a, vec_b, vec_c, sym, color);
            return;
        }

        let dx = (vec_c.x - vec_a.x)*(vec_b.y - vec_a.y)/(vec_c.y - vec_a.y);
        let dz_dx = (vec_c.z - vec_a.z)/(vec_c.x - vec_a.x);

        let midpoint = Vec3 { x: vec_a.x + dx , y: vec_b.y, z: vec_a.z + dz_dx * dx};

        self.draw_flat_top_triangle(&vec_c, &midpoint, &vec_b, sym, color);
        self.draw_flat_bottom_triangle(&vec_a, &midpoint, &vec_b, sym, color);
    }

    fn write_to_buffer(&mut self, r: usize, c: usize, z: f32, sym: char, color: Color)
    {
        
        let index = HEADER_SIZE + r*(T_COLS + 1) + c;
        if index >= BUFFER_SIZE // || !z.is_finite() || z <= 0.0
        {
            return;
        } 

        // let inv_z = 1.0/z;

        // if inv_z < self.depth_buffer[index]
        // {
        //     return;
        // }

        
        // self.depth_buffer[index] = inv_z.abs();
        self.character_buffer[index] = sym;
        self.color_buffer[index] = color;
    }

    pub fn flush_buffer(&mut self)
    {
        for i in 0..T_ROWS
        {
            self.character_buffer[HEADER_SIZE + i*(T_COLS + 1) + T_COLS] = '\n';
        }

        let header_output = self.character_buffer[0..HEADER_SIZE].iter().collect::<String>();

        let meshed_output = self.color_buffer[HEADER_SIZE..].iter().zip(self.character_buffer[HEADER_SIZE..].iter()).map(|(color, c)| color.get_ansii_code() + &c.to_string()).collect::<String>();

        let output = header_output + &meshed_output;


        let mut std_out = std::io::stdout();
        write!(std_out, "{}", output).unwrap();
        std_out.flush().unwrap();

        self.character_buffer[HEADER_SIZE..].fill(' ');
        self.color_buffer[HEADER_SIZE..].fill(Color::Reset);
        self.depth_buffer[HEADER_SIZE..].fill(0.0);
    }

    pub fn add_mesh(&mut self, mesh: Mesh) -> usize
    {
        self.meshes.push(Rc::new(RefCell::new(mesh)));
        self.meshes.len() - 1   
    }

    pub fn render(&mut self)
    {
        for mesh_idx in 0..self.meshes.len()
        {
            let mesh_wrapper = Rc::clone(&self.meshes[mesh_idx]);

            self.draw_mesh(&mesh_wrapper.borrow());
        }
    }
}