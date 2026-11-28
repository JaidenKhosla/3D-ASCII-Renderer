use std::{collections::HashMap, fs::File, hash::{BuildHasher, Hash, Hasher, RandomState}, io::{BufRead, BufReader, Read, Seek}, path::Path, ptr::null};

use crate::{Color::{self, RGB}, Mesh, Vec3};

pub struct STL {}



impl STL
{
    pub fn from_stl(path: &Path) -> Mesh
    {
        let file = File::open(path).unwrap();
        let mut buff_reader = BufReader::new(file);

        let mut triangles: Vec<[usize; 3]> = vec![];
    

        let mut point2index: HashMap<Vec3, usize> = HashMap::new();

        let mut curr_triangle: Option<&mut [usize; 3]> = None;
        let mut triangle_idx = 0usize;

        let lines = buff_reader.lines().collect::<Vec<Result<String, _>>>();

        let mut triangle_size = 0usize;

        for unparsed_line in lines
        {
            let line = unparsed_line.unwrap().trim().to_lowercase().to_string();
            
            if line == "outer loop"
            {
                triangles.push([0usize; 3]);
                triangle_size+=1;

                curr_triangle = Some(&mut triangles[triangle_size - 1]);
                triangle_idx = 0;
            }
            else if line.starts_with("vertex")
            {
                let split_line = line.split(" ").map(|s| s.to_string()).collect::<Vec<String>>();

                let splice =  &split_line[1..].iter().map(|str| str.parse::<f32>().unwrap()).collect::<Vec<f32>>();
               
                let vector = Vec3 { x: splice[0], y: splice[1], z: splice[2] };

                if !point2index.contains_key(&vector) {
                    point2index.insert(vector, point2index.len());
                }
                
                match &mut curr_triangle
                {
                    Some(arr) => {
                        (*arr)[triangle_idx] = *point2index.get(&vector).unwrap();
                        triangle_idx+=1;
                    },
                    
                    _ => {}
                }
            }

            
        }

        let zero = Vec3 { x: 0.0, y: 0.0, z: 0.0};

        let mut vertices: Vec<Vec3> = Vec::with_capacity(point2index.len());

        for _ in 0..vertices.capacity()
        {
            vertices.push(zero);
        }

        for (k,v) in point2index.iter()
        {
            vertices[*v] = *k;
        }

        
        let symbols = ".'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$".chars().collect::<Vec<char>>();

        Mesh {
            vertices,
            triangles,
            colors: (0..triangle_size).map(|_| Color::BrightRed).collect(),
            symbols: (0..triangle_size).map(|_| symbols[random(symbols.len() as u8) as usize]).collect(),

            translation: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3]
        }
        
    }
}


pub fn random(ceiling: u8) -> u8
{
    (RandomState::new().build_hasher().finish()%ceiling as u64) as u8
}