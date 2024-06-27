use bevy::{
    prelude::*,
    render::{
	//texture::{FallbackImage},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssetUsages,
        render_asset::RenderAssets,
        //render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        //renderer::{RenderContext, RenderDevice},
        //Render, RenderApp, RenderSet,
    },
};

const MAX_DEPTH: i32 = 10;

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
    pub fn new(max_corner: Vec3,min_corner: Vec3,child_index: u32,object_index: u32) -> Self {
	let centre = (max_corner+min_corner) * 0.5;

	Self {
	    max_corner,
	    min_corner,
	    centre,
	    child_index,
	    object_index,
	    object_count: 0,
	}
    }

    pub fn grow_to_include(&mut self,p: Vec3) {
	self.min_corner = self.min_corner.min(p);
	self.max_corner = self.max_corner.max(p);
    }

    pub fn grow_to_include_object(&mut self, obj: &Object) {
	match obj.ty as u32 {
	    0 => {
		let sphere_max = obj.pos + obj.params[0];
		let sphere_min = obj.pos - obj.params[0];

		self.grow_to_include(sphere_max);
		self.grow_to_include(sphere_min);
	    },

	    1 => {
		self.grow_to_include(Vec3::new(obj.params[0],obj.params[1],obj.params[2]));
		self.grow_to_include(Vec3::new(obj.params[3],obj.params[4],obj.params[5]));
		self.grow_to_include(Vec3::new(obj.params[6],obj.params[7],obj.params[8]));
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
    pub fn split(&mut self,node:&mut Node,depth: i32) {
	if depth == MAX_DEPTH {
	    return;
	}

	let aabb_size = node.max_corner-node.min_corner;
	let split_axis = if aabb_size.x > aabb_size.y.max(aabb_size.z) {0} else {if aabb_size.y > aabb_size.z {1} else {2}};
	let split_pos = node.centre[split_axis];

	node.child_index = self.bvh.len() as u32;
	let mut child_a = Node::new(Vec3::new(0.0,0.0,0.0),Vec3::new(0.0,0.0,0.0),0,node.object_index);
	let mut child_b = Node::new(Vec3::new(0.0,0.0,0.0),Vec3::new(0.0,0.0,0.0),0,node.object_index);

	for i in node.object_index..node.object_count {
	    let object = &self.scene[i as usize];
	    let side_a = object.pos[split_axis] < split_pos;
	    let child = if side_a {&mut child_a} else {&mut child_b};
	    child.grow_to_include_object(object);
	    child.object_count += 1;

	    if side_a {
		let temp = (child.object_index + child.object_count - 1) as usize;
		let swap = self.scene[i as usize].clone();
		self.scene[i as usize] = self.scene[temp];
		self.scene[temp] = swap; 
	    }
	}

	self.split(&mut child_a,depth+1);
	self.split(&mut child_b,depth+1);
    }
}
