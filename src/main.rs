use std::{cell::RefCell, io::Write, sync::{LazyLock, Mutex}, rc::Rc, time::Duration, cell::Ref};

#[repr(u8)]
#[derive(Clone, Copy)]
#[allow(unused)]
enum Color
{
    Color(u8),
    Color256(u8),
    RGB(u8, u8, u8),

    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan, 
    White,

    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,

    Reset
}

impl Color
{
    fn id(&self) -> u8
    {
        match self
        {
            Color::Color(code) => *code, 
            Color::Black => 30,
            Color::Red => 31,
            Color::Green => 32,
            Color::Yellow => 33,
            Color::Blue => 34,
            Color::Magenta => 35,
            Color::Cyan => 36,
            Color::White => 37,

            Color::BrightBlack => 90,
            Color::BrightRed => 91,
            Color::BrightGreen => 92,
            Color::BrightYellow => 93,
            Color::BrightBlue => 94,
            Color::BrightMagenta => 95,
            Color::BrightCyan => 96,
            Color::BrightWhite => 97,

            _ => 0
        }
    }

    fn get_ansii_code(&self) -> String
    {
        match self
        {
            Color::Color256(val) => format!("\x1b[38;5;{}m", val),
            Color::RGB(r,g,b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
            _ => format!("\x1b[{}m", self.id())
        }
    }
}

struct Vec2 { x: f32, y: f32}
struct Vec3 { x: f32, y: f32, z: f32}

impl Vec3
{
    fn rotate_x(&self, rad: f32) -> Vec3
    {
        let cos = rad.cos();
        let sin = rad.sin();

        Vec3 { x: self.x, y: self.y*cos - self.z*sin, z: self.y*sin + self.z*cos}
    }

    fn rotate_y(&self, rad: f32) -> Vec3
    {
        let cos = rad.cos();
        let sin = rad.sin();

        Vec3 { x: self.x*cos + self.z * sin, y: self.y, z: -self.x*sin + self.z*cos}
    }

    fn rotate_z(&self, rad: f32) -> Vec3
    {
        let cos = rad.cos();
        let sin = rad.sin();

        Vec3 { x: self.x*cos - self.y*sin, y: self.x*sin + self.y*cos, z: self.z}
    }

    fn project(&self) -> Vec2
    {
        Vec2 { x: (self.x/self.z + T_COLS as f32 / 2.0).round(), y: (self.y/self.z + T_ROWS as f32 / 2.0).round() }
    }

    fn translate(&self, dx: f32, dy: f32, dz: f32) -> Vec3
    {
        Vec3{ x: self.x + dx, y: self.y + dy, z: self.z + dz}
    }

    fn scale(&self, dx: f32, dy: f32, dz: f32) -> Vec3
    {
        Vec3 { x: self.x * dx, y: self.y * dy, z: self.z * dz}
    }

    fn dot_product(a: &Vec3, b: &Vec3) -> f32
    {
        a.x * b.x + a.y * b.y + a.z * b.z
    }

    fn cross_product(a: &Vec3, b: &Vec3) -> Vec3
    {
        Vec3 { x: a.y * b.z - a.z*b.y, y: a.z*b.x-a.x*b.z, z: a.x*b.y-a.y*b.x}
    }
}

type Triangle = [usize; 3];
struct Mesh
{
    vertices: [Vec3; 8],
    triangles: [Triangle; 12],
    colors: [Color; 12],
    symbols: [char; 12],

    translation: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3]
}

impl Mesh
{
    fn update_rotation(&mut self)
    {
        self.rotation[0] += 0.05;
        self.rotation[1] += 0.05;
        self.rotation[2] += 0.05;
    }

}

struct Renderer
{
    character_buffer: [char; BUFFER_SIZE],
    color_buffer: [Color; BUFFER_SIZE],
    depth_buffer: [f32; BUFFER_SIZE],
    meshes: Vec<Rc<RefCell<Mesh>>>
}

impl Renderer
{
    fn new() -> Self
    {
        let mut character_buffer = [' '; BUFFER_SIZE];
        let color_buffer = [Color::Reset; BUFFER_SIZE];
        let depth_buffer = [0.0; BUFFER_SIZE];

        character_buffer[0] = '\x1b';
        character_buffer[1] = '[';
        character_buffer[2] = 'H';

        Renderer { character_buffer, color_buffer, depth_buffer: depth_buffer, meshes: Vec::new() }
    }

    fn draw_mesh(&mut self, mesh: &Mesh)
    {   
        for triangle_idx in 0..mesh.triangles.len()
        {   
            let triangle = mesh.triangles[triangle_idx];
            
            let sym = mesh.symbols[triangle_idx];
            let color = mesh.colors[triangle_idx];

            let modified_triangle = triangle.map(|idx| mesh.vertices[idx].rotate_x(mesh.rotation[0]).rotate_y(mesh.rotation[1]).rotate_z(mesh.rotation[2]).translate(0.0, 0.0, 8.0).scale(72.0, 24.0, 1.0));
            
            let d0 = Vec3 { x: modified_triangle[1].x - modified_triangle[0].x, y: modified_triangle[1].y - modified_triangle[0].y, z: modified_triangle[1].z - modified_triangle[0].z};
            let d1 = Vec3 { x: modified_triangle[2].x - modified_triangle[0].x, y: modified_triangle[2].y - modified_triangle[0].y, z: modified_triangle[2].z - modified_triangle[0].z};
            
            let cross = Vec3::cross_product(&d0, &d1);
            let camera_dot = Vec3::dot_product(&cross, &CAMERA_POS);

            //back face culling
            if camera_dot >= 0.0
            {
                continue;
            }


            let [projected_vec0, projected_vec1, projected_vec2] = modified_triangle.map(|vec_i| vec_i.project());

            self.draw_triangle(&projected_vec0, &projected_vec1, &projected_vec2, sym, color);
        }
    }

    fn draw_line(&mut self, x1: usize, x2: usize, y: usize, sym: char, color: Color)
    {
        for x in x1.min(x2)..=x1.max(x2)
        {
            self.write_to_buffer(y, x, sym, color);
        }
    }


    fn draw_flat_bottom_triangle(&mut self, top: &Vec2, b1: &Vec2, b2: &Vec2, sym: char, color: Color)
    {
        let slope1 = (top.x - b1.x)/(top.y - b1.y);
        let slope2 = (top.x - b2.x)/(top.y - b2.y);

        let delta_y = (b1.y - top.y).round() as usize;

        let mut x1 = top.x;
        let mut x2 = top.x;

        for y in 0..=delta_y
        {
            self.draw_line(x1.round() as usize, x2.round() as usize, top.y.round() as usize + y, sym, color);

            x1 += slope1;
            x2 += slope2;
        }
    }

    fn draw_flat_top_triangle(&mut self, bottom: &Vec2, t1: &Vec2, t2: &Vec2, sym: char, color: Color)
    {  
        let slope1 = (bottom.x - t1.x)/(bottom.y - t1.y);
        let slope2 = (bottom.x - t2.x)/(bottom.y - t2.y);

        let delta_y = (bottom.y - t1.y).round() as usize;
        
        let mut x1 = bottom.x;
        let mut x2 = bottom.x;

        for y in 0..=delta_y
        {
            let y_level = bottom.y.round() as i32 - y as i32;

            if y_level >= 0
            {
                self.draw_line(x1.round() as usize, x2.round() as usize, y_level as usize, sym, color);
            }

            x1 -= slope1;
            x2 -= slope2;
        }
    }

    fn draw_triangle(&mut self, a: &Vec2, b: &Vec2, c: &Vec2, sym: char, color: Color)
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

        let midpoint = Vec2 { x: vec_a.x + (vec_c.x - vec_a.x)*(vec_b.y - vec_a.y)/(vec_c.y - vec_a.y), y: vec_b.y};

        self.draw_flat_top_triangle(&vec_c, &midpoint, &vec_b, sym, color);
        self.draw_flat_bottom_triangle(&vec_a, &midpoint, &vec_b, sym, color);
    }

    fn write_to_buffer(&mut self, r: usize, c: usize, sym: char, color: Color)
    {
        
        let index = HEADER_SIZE + r*(T_COLS + 1) + c;
        if index >= BUFFER_SIZE
        {
            return;
        } 

        self.character_buffer[index] = sym;
        self.color_buffer[index] = color;
    }

    fn flush_buffer(&mut self)
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
    }

    fn add_mesh(&mut self, mesh: Mesh) -> usize
    {
        self.meshes.push(Rc::new(RefCell::new(mesh)));
        self.meshes.len() - 1   
    }

    fn render(&mut self)
    {
        for mesh_idx in 0..self.meshes.len()
        {
            let mesh_wrapper = Rc::clone(&self.meshes[mesh_idx]);

            self.draw_mesh(&mesh_wrapper.borrow());
        }
    }
}



const T_ROWS: usize = 36;
const T_COLS: usize = 165;

const HEADER_SIZE: usize = 3;
const BUFFER_SIZE: usize = T_ROWS *(T_COLS + 1) + HEADER_SIZE;



// static BUFFER: LazyLock<Mutex<[char; BUFFER_SIZE]>> = LazyLock::new(|| {
//     Mutex::new([' '; BUFFER_SIZE])
// });

// static COLOR_BUFFER: LazyLock<Mutex<[Color; BUFFER_SIZE]>> = LazyLock::new(|| {
//     Mutex::new([Color::Red; BUFFER_SIZE])
// });

// static DEPTH_BUFFER: LazyLock<Mutex<[f32; BUFFER_SIZE]>> = LazyLock::new(||{
//     Mutex::new([0.0; BUFFER_SIZE])
// });

// const FPS: u32 = 60;

const CAMERA_POS: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 1.0};



fn main() {
    
    let mut MAIN_CUBE: Mesh = Mesh {
        vertices: [
            Vec3 { x: -1.0, y: -1.0, z: -1.0},
            Vec3 { x: -1.0, y: 1.0, z: -1.0},
            Vec3 { x: 1.0, y: 1.0, z: -1.0},
            Vec3 { x: 1.0, y: -1.0, z: -1.0},
            Vec3 { x: 1.0, y: 1.0, z: 1.0},
            Vec3 { x: 1.0, y: -1.0, z: 1.0},
            Vec3 { x: -1.0, y: -1.0, z: 1.0},
            Vec3 { x: -1.0, y: 1.0, z: 1.0}
        ],
    
        triangles: [
            [0, 1, 2],
            [0, 2, 3],
            [3, 2, 4],
            [3, 4, 5],
            [5, 4, 7],
            [5, 7, 6],
            [6, 7, 1],
            [6, 1, 0],
            [6, 0, 3],
            [6, 3, 5],
            [1, 7, 4],
            [1, 4, 2]
        ],
    
        colors: [ Color::RGB(255,0,0), Color::RGB(255,0,0), Color::Blue, Color::Blue, Color::Green, Color::Green, Color::Yellow, Color::Yellow, Color::Magenta, Color::Magenta, Color::Cyan, Color::Cyan],
    
        symbols: ['@', '@', '#', '#', '$', '$', '%', '%', '&', '&', '*', '*'],
    
        translation: [0.0,0.0,0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0]
    };
    
    print!("\x1b[2J\x1b[3J"); //clears screen
    std::io::stdout().flush().unwrap();
    
    let mut renderer = Renderer::new();

    let idx = renderer.add_mesh(MAIN_CUBE);
    
    loop
    {
        print!("\x1b[3J");
        std::io::stdout().flush().unwrap();
        // draw_flat_bottom_triangle(&Vec2{ x: 5.0, y: 0.0},&Vec2{ x: 0.0, y: 5.0}, &Vec2{ x: 10.0, y: 5.0});
        // draw_flat_top_triangle(&Vec2 { x: 5.0, y: 10.0 }, &Vec2{ x: 0.0, y: 5.0 }, &Vec2 { x: 10.0, y: 5.0});
        // draw_triangle(&Vec2{ x: 0.0, y: 10.0 }, &Vec2{ x: 20.0, y: 0.0 }, &Vec2{ x: 10.0, y: 20.0 });
        
        renderer.render();
        renderer.flush_buffer();
        std::thread::sleep(Duration::from_millis(500));
        
        let mut cube = renderer.meshes[idx].borrow_mut();

        cube.update_rotation();

    }
}
