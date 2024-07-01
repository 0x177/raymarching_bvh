use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource},
        render_resource::*,
    },
};

const MAX_DEPTH: i32 = 2;

#[derive(Resource)]
pub struct RayMarcherBVHBindGroup(pub BindGroup);

#[derive(Copy,ShaderType,Clone,Resource)]
pub struct Object {
    pub pos: Vec3,
    pub ty: f32,
    pub params: [f32; 12]
}

#[derive(Debug,Copy,ShaderType,Clone,Resource)]
pub struct BvhCamera {
    pub pos: Vec3,
    pub rot: Vec2
}

#[derive(Debug,Copy,ShaderType,Clone,Resource)]
pub struct Node {
    pub max_corner: Vec3,
    pub min_corner: Vec3,
    pub centre: Vec3,
    pub child_index: u32,
    pub object_index: u32,
    pub object_count: u32,
}

impl Node {
    pub fn new(max_corner: Vec3,min_corner: Vec3,object_index: u32,object_count: u32) -> Self {
	let centre = (max_corner+min_corner) * 0.5;

	Self {
	    max_corner,
	    min_corner,
	    centre,
	    child_index: 0,
	    object_index,
	    object_count,
	}
    }

    pub fn grow_to_include(&mut self,p: Vec3) {
	self.min_corner = self.min_corner.min(p);
	self.max_corner = self.max_corner.max(p);

	self.centre = (self.max_corner + self.min_corner) * 0.5;
    }

    pub fn grow_to_include_object(&mut self, obj: &Object) {
	match obj.ty as u32 {
	    0 => {
		let sphere_max = obj.pos + obj.params[0];
		let sphere_min = obj.pos - obj.params[0];

		self.grow_to_include(sphere_max);
		self.grow_to_include(sphere_min);

		//self.object_count += 1;
	    },

	    1 => {
		self.grow_to_include(Vec3::new(obj.params[0],obj.params[1],obj.params[2]));
		self.grow_to_include(Vec3::new(obj.params[3],obj.params[4],obj.params[5]));
		self.grow_to_include(Vec3::new(obj.params[6],obj.params[7],obj.params[8]));
		//self.object_count += 1;
	    }
	    
	    _ => {
		panic!("attempet to grow to include unsupported type");
	    }
	}
    }
}

#[derive(Resource,Component, Clone, Deref, ExtractResource,AsBindGroup)]
pub struct RayMarcherData {
    #[storage(0,visibility(compute),read_only)]
    #[deref]
    pub bvh: Vec<Node>,
    #[storage(1,visibility(compute),read_only)]
    pub scene: Vec<Object>,
    #[uniform(2)]
    pub camera: BvhCamera
}

impl RayMarcherData {
    pub fn split(&mut self,index: u32,depth: i32) {
	if depth == MAX_DEPTH {
	    return;
	}
	
	let node = self.bvh[index as usize].clone();

	let size = node.max_corner - node.min_corner;

	let axis = if size.x > size.y.max(size.z) {0} else {if size.y > size.z {1} else {2}};
	let split_pos = node.centre[axis];

	let mut child_a = Node::new(Vec3::new(0.0,0.0,0.0),Vec3::new(0.0,0.0,0.0),node.object_index,0);
	self.bvh[index as usize].child_index = self.bvh.len() as u32;
	let mut child_b = Node::new(Vec3::new(0.0,0.0,0.0),Vec3::new(0.0,0.0,0.0),node.object_index,0);
	
	for i in node.object_index..node.object_index+node.object_count {
	    let obj = self.scene[i as usize];
	    let in_a = obj.pos[axis] < split_pos;
	    let child = if in_a {&mut child_a} else {&mut child_b};
	    child.object_count += 1;
	    child.grow_to_include_object(&obj);
	    if in_a {
		let swap_index = child.object_index + child.object_count - 1;
		self.scene.swap(i as usize,swap_index as usize);
		child_b.object_index += 1;
	    }
	}

	self.bvh.push(child_a);
	self.bvh.push(child_b);

	self.split(self.bvh[index as usize].child_index,depth+1);
	self.split(self.bvh[index as usize].child_index + 1,depth+1);
    }
}
