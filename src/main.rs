mod stl;
pub mod renderer;

use std::{cell::{Ref, RefCell}, f32::consts::PI, fmt::Display, hash::{Hash, Hasher}, io::Write, path::Path, rc::Rc, sync::{LazyLock, Mutex}, time::{Duration, Instant}};
use crate::stl::STL;
use crate::renderer::Renderer;


const T_ROWS: usize = 36;
const T_COLS: usize = 165;

const HEADER_SIZE: usize = 3;
const BUFFER_SIZE: usize = T_ROWS *(T_COLS + 1) + HEADER_SIZE;

const CAMERA_POS: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 1.0};


#[repr(u8)]
#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy)]
struct Vec2 { x: f32, y: f32}

impl Vec2
{
    fn cross_product(a: &Vec2, b: &Vec2) -> Vec2 
    {
        Vec2 { x: a.x * b.y, y: - a.y * b.x }
    }

    fn dot_product(a: &Vec2, b: &Vec2) -> f32
    {
        a.x * b.x + a.y * b.y
    }

    fn extend(&self, z: f32) -> Vec3
    {
        Vec3 { x: self.x, y: self.y, z }
    }
}

#[derive(Clone, Copy, Debug)]
struct Vec3 { x: f32, y: f32, z: f32}

impl Display for Vec3
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vec3({}, {}, {})", self.x, self.y, self.z)
    }
}

impl PartialEq<Vec3> for Vec3
{
    fn eq(&self, other: &Vec3) -> bool
    {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl Eq for Vec3 {}

impl Hash for Vec3
{
   fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_string().hash(state);
    }
}

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
        Vec2 { x: (self.x/self.z + T_COLS as f32 / 2.0).round(), y: (-self.y/self.z + T_ROWS as f32 / 2.0).round() }
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

#[derive(Debug)]
struct Mesh
{
    vertices: Vec<Vec3>,
    triangles: Vec<Triangle>,
    colors: Vec<Color>,
    symbols: Vec<char>,

    translation: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3]
}

impl Mesh
{
    //I hate linear algebra

    // fn calculate_normals(&mut self)
    // {
    //     for triangle_idx in 0..self.triangles.len()
    //     {
    //         let triangle = self.triangles[triangle_idx];

    //         let projected_points = triangle.map(|idx| self.vertices[idx]);

    //         let mut centroid = Vec2 { x: 0.0, y: 0.0 };
            
    //         for projected_point in projected_points.iter()
    //         {
    //             centroid.x += projected_point.x; centroid.y += projected_point.y;
    //         }

    //         let size = projected_points.len() as f32;

    //         centroid.x/=size; centroid.y/=size;

    //         let angles = projected_points.map(|point| f32::atan2(point.x - centroid.x, point.y - centroid.y)).map(|angle| if angle < 0.0 { angle + 2.0*PI } else { angle} );

            
    //         let mut join: Vec<(usize, f32)> = triangle.iter().map(|a| *a).zip(angles).collect::<Vec<(usize, f32)>>();

    //         join.sort_by(|a,b| a.1.total_cmp(&b.1));

    //         println!("{:?} -> {:?}", triangle, join);

    //         for vertice_idx in 0..3
    //         {
    //            self.triangles[triangle_idx][vertice_idx] = join[vertice_idx].0;
    //         }
    //     }
    
}



struct FPSManager 
{
    target_fps: u32,
    last_frame: Instant,
    target_time: Duration
}

impl FPSManager
{
    fn get_deltatime(&mut self) -> f32
    {
        let delta = self.last_frame.elapsed();
        
        
        if let Some(sleep) = self.target_time.checked_sub(delta)
        {
            std::thread::sleep(sleep);
        }
        
        let elapsed = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();

        elapsed

    
    }
}





// static BUFFER: LazyLock<Mutex<[char; BUFFER_SIZE]>> = LazyLock::new(|| {
//     Mutex::new([' '; BUFFER_SIZE])
// });

// static COLOR_BUFFER: LazyLock<Mutex<[Color; BUFFER_SIZE]>> = LazyLock::new(|| {
//     Mutex::new([Color::Red; BUFFER_SIZE])
// });

// static DEPTH_BUFFER: LazyLock<Mutex<[f32; BUFFER_SIZE]>> = LazyLock::new(||{
//     Mutex::new([0.0; BUFFER_SIZE])
// });

const FPS: u32 = 60;

static FPS_MANAGER: LazyLock<Mutex<FPSManager>> = LazyLock::new(|| Mutex::new(FPSManager { target_fps: FPS, last_frame: Instant::now(), target_time: Duration::from_secs_f32(1.0/FPS as f32) }));


fn main() {
    
    let mut main_cube: Mesh = Mesh {
        vertices: [
            Vec3 { x: -1.0, y: -1.0, z: -1.0},
            Vec3 { x: -1.0, y: 1.0, z: -1.0},
            Vec3 { x: 1.0, y: 1.0, z: -1.0},
            Vec3 { x: 1.0, y: -1.0, z: -1.0},
            Vec3 { x: 1.0, y: 1.0, z: 1.0},
            Vec3 { x: 1.0, y: -1.0, z: 1.0},
            Vec3 { x: -1.0, y: -1.0, z: 1.0},
            Vec3 { x: -1.0, y: 1.0, z: 1.0}
        ].to_vec(),
    
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
        ].to_vec(),
    
        colors: [ Color::RGB(255,0,0), Color::RGB(255,0,0), Color::Blue, Color::Blue, Color::Green, Color::Green, Color::Yellow, Color::Yellow, Color::Magenta, Color::Magenta, Color::Cyan, Color::Cyan].to_vec(),
    
        symbols: ['@', '@', '#', '#', '$', '$', '%', '%', '&', '&', '*', '*'].to_vec(),
    
        translation: [0.0,0.0,0.0],
        rotation: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0]
    };

    let mut main_pyramid = Mesh {
        vertices: [
            Vec3 { x: -1.0, y: 1.0, z: -1.0 },
            Vec3 { x: 1.0, y: 1.0, z: -1.0},
            Vec3 { x: -1.0, y: 1.0, z: 1.0},
            Vec3 { x: 1.0, y: 1.0, z: 1.0},
            Vec3 { x: 0.0, y: -1.0, z: 0.0}
        ].to_vec(),

        triangles: [
            [0, 1, 2],
            [1, 3, 2],
            [0, 4, 1],
            [0, 2, 4],
            [2, 3, 4],
            [1, 4, 3]
        ].to_vec(),

        colors: [ Color::Blue, Color::Blue, Color::Green, Color::Yellow, Color::Red, Color::Magenta].to_vec(),

        symbols: [ '@', '@', '#', '$', '%', '&'].to_vec(),

        translation: [0.0, 4.0, 0.0],
        rotation: [0.0; 3],
        scale: [1.0; 3]
    };

    let import = STL::from_stl(&Path::new("./output.txt"));
    
    println!{"{:?}", main_pyramid.triangles}
    
    
    let mut renderer = Renderer::new();
    
    // let cube_idx = renderer.add_mesh(main_cube);
    // let pyramid_idx = renderer.add_mesh(main_pyramid);
    // let import
    
    let import_idx = renderer.add_mesh(import);

    static FULL_ROTATION: f32 = PI*2.0;
    
    
    print!("\x1b[2J\x1b[3J\x1b[?25l"); //clears screen
    std::io::stdout().flush().unwrap();
    
    let mut angle = 0.0;

    loop
    {
        let delta = FPS_MANAGER.lock().unwrap().get_deltatime();

        print!("\x1b[3J");
        std::io::stdout().flush().unwrap();
        // draw_flat_bottom_triangle(&Vec2{ x: 5.0, y: 0.0},&Vec2{ x: 0.0, y: 5.0}, &Vec2{ x: 10.0, y: 5.0});
        // draw_flat_top_triangle(&Vec2 { x: 5.0, y: 10.0 }, &Vec2{ x: 0.0, y: 5.0 }, &Vec2 { x: 10.0, y: 5.0});
        // draw_triangle(&Vec2{ x: 0.0, y: 10.0 }, &Vec2{ x: 20.0, y: 0.0 }, &Vec2{ x: 10.0, y: 20.0 });
        
        renderer.render();
        renderer.flush_buffer();
        
        let mut cube = renderer.get_mesh(import_idx);
        // let mut pyramid = renderer.meshes[pyramid_idx].borrow_mut();
        // let mut import_mesh = 

        let angle_update = PI/8.0 * delta;

        angle += angle_update;

        // cube.rotation[0] = (cube.rotation[0] + angle_update)%FULL_ROTATION;
        // cube.rotation[1] = (cube.rotation[1] + angle_update)%FULL_ROTATION;
        // cube.rotation[2] = (cube.rotation[2] + angle_update)%FULL_ROTATION;

        // cube.translation[1] += 5.0*angle.sin() * delta;
        
        // pyramid.rotation[0] = (pyramid.rotation[0] + angle_update)%FULL_ROTATION;
        // pyramid.rotation[1] = (pyramid.rotation[1] + angle_update)%FULL_ROTATION;
        // pyramid.rotation[0] = (pyramid.rotation[0] + 2.0*angle_update)%FULL_ROTATION;
        // cube.rotation[1] += PI/8.0*delta;
        // cube.rotation[2] += PI/8.0*delta;



    }
}
