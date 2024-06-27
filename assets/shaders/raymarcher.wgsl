@group(0) @binding(0) var texture: texture_storage_2d<rgba8unorm, read_write>;

struct Object {
  pos: vec3<f32>,
  ty: f32,
  params: array<f32, 12>
};

struct Camera {
  pos: vec3<f32>,
  rot: vec2<f32>
};

struct Node {
  max_corner: vec3<f32>,
  min_corner: vec3<f32>,
  centre: vec3<f32>,
  child_index: u32,
  object_index: u32,
  object_count: u32
};

@group(1) @binding(0) var<storage, read> bvh: array<Node>;
@group(1) @binding(1) var<storage, read> scene: array<Object>;
@group(1) @binding(2) var<uniform> camera: Camera;

struct Ray {
  ro: vec3<f32>,
  rd: vec3<f32>,
  ird: vec3<f32>
};

@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let uv = vec2<f32>(f32(invocation_id.x), f32(invocation_id.y))/vec2<f32>(1920,1080);
    
    textureStore(texture, location, vec4(uv,1.0,1.0));
}

fn rot_2d(angle: f32) -> mat2x2<f32> {
  let sin_angle = sin(angle);
  let cos_angle = cos(angle);

  return mat2x2<f32>(
	      cos_angle, -sin_angle,
	      sin_angle, cos_angle
	      );
}

fn dot2(v: vec3<f32>) -> f32 { return dot(v,v); }

fn triangle(p: vec3<f32>,a: vec3<f32>,b: vec3<f32>,c: vec3<f32>) -> f32 {
  let ba = b - a;
  let pa = p - a;
  let cb = c - b;
  let pb = p - b;
  let ac = a - c;
  let pc = p - c;
  let nor = cross( ba, ac );

  var fuck_wgsl = 0.0;

  let cond = sign(dot(cross(ba,nor),pa)) + sign(dot(cross(cb,nor),pb)) +sign(dot(cross(ac,nor),pc))<2.0;
  
  if (cond) {
    fuck_wgsl = min( min(dot2(ba*clamp(dot(ba,pa)/dot2(ba),0.0,1.0)-pa),dot2(cb*clamp(dot(cb,pb)/dot2(cb),0.0,1.0)-pb) ),dot2(ac*clamp(dot(ac,pc)/dot2(ac),0.0,1.0)-pc) );
  } else {
    fuck_wgsl = dot(nor,pa)*dot(nor,pa)/dot2(nor);
  }
  
  return sqrt(fuck_wgsl);
}

fn object_distance(p: vec3<f32>,op: vec3<f32>,ty: f32,params: array<f32, 12>) -> f32 {
  var dist: f32 = 9999.0;

  if ty == 0.0 {
      dist = length(p-op)-params[0];
 } else if ty == 1.0 {
      dist = triangle(p,vec3<f32>(params[0],params[1],params[2]),vec3<f32>(params[3],params[4],params[5]),vec3<f32>(params[6],params[7],params[8]));
 }

  return dist;
}

fn ray_aabb(min_corner: vec3<f32>,max_corner: vec3<f32>, ray: Ray) -> vec2<f32> {
  let tMin = (min_corner - ray.ro) * ray.ird;
  let tMax = (max_corner - ray.ro) * ray.ird;
  let t1 = min(tMin, tMax);
  let t2 = max(tMin, tMax);
  let tNear = max(max(t1.x, t1.y), t1.z);
  let tFar = min(min(t2.x, t2.y), t2.z);
  //return tNear < tFar && tFar > 0.0;
  return vec2<f32>(tNear,tFar);
}

fn traverse_bvh(ray: Ray) -> vec2<f32> {
    var stack: array<u32,10> = array<u32,10>();
    var index = 0;
    stack[index] = u32(0);
    index += 1;

    var min_dist = 999999.0;
    var material = 0.0;

    while (index > 0) {
	index -= 1;
	let node_index = stack[index];
	let node = bvh[node_index];

	let intersection = ray_aabb(node.min_corner,node.max_corner,ray);
	
	//if intersection.x < min_dist {
	if node.child_index == 0 {
	    for (var i = node.object_index; i < node.object_index + node.object_count; i++) {
		let object = scene[i];
		let dist = object_distance(ray.ro,object.pos,object.ty,object.params);
		min_dist = min(min_dist,dist);
	    }
	} else {
	    let child_index_a: u32 = node.child_index;
	    let child_a = bvh[child_index_a];
	    //index += 1;
	    let child_index_b: u32 = node.child_index + 1;
	    let child_b = bvh[child_index_b];
	    //index += 1;

	    let dist_a = ray_aabb(child_a.min_corner,child_a.max_corner,ray).x;
	    let dist_b = ray_aabb(child_b.min_corner,child_b.max_corner,ray).x;

	    let a_is_nearest = dist_a < dist_b;
	    // WHY THE FUCK DOES THIS LANGUAGE NOT HAVE A TURNARY OPERATOR OR AN EQUEVELANT ONE LINER
	    var near = 0.0;
	    var far = 0.0;
	    
	    if (a_is_nearest) {
		near = dist_a;
		far = dist_b;
	    } else {
		near = dist_b;
		far = dist_a;
	    }

	    var index_near = 0;
	    var index_far = 0;

	    if (a_is_nearest) {
		index_near = child_index_a;
		index_far = child_index_b;
	    } else {
		index_near = child_index_a;
		index_far = child_index_b;
	    }
	    
	}  
	//  } 
    }
    
    return vec2<f32>(min_dist,material);
}

fn get_distance(ray:Ray) -> vec2<f32> {
  var d = -ray.ro.y+8.0;
  var m = 0.0;

  let traverse = traverse_bvh(ray);
  d = traverse.x;
  m = traverse.y;
  
  return vec2<f32>(d,m);
}

fn get_normal(ray:Ray) -> vec3<f32> {
  let distance = get_distance(ray).x;
    let e = vec2<f32>(0.01,0.0);
    
    let normal = distance - vec3<f32>(
				      get_distance(Ray(ray.ro-e.xyy,ray.rd,ray.ird)).x,
				      get_distance(Ray(ray.ro-e.yxy,ray.rd,ray.ird)).x,
				      get_distance(Ray(ray.ro-e.yyx,ray.rd,ray.ird)).x,
    );

    return normalize(normal);
}

fn ray_march(ray: ptr<function,Ray>,max_steps:i32,max_distance:f32,surface_distance:f32) -> vec2<f32> {
    var distance_marched: f32 = 0.0;
    
    var material = 0.0;

    for (var i = 0; i < max_steps; i += 1) {
        let distance_to_scene = get_distance(*(ray));
        distance_marched += distance_to_scene.x;

        material = distance_to_scene.y;

        if (distance_marched>max_distance || distance_to_scene.x<surface_distance) {break;}
	(*ray).ro += (*ray).rd*distance_marched;
    }
    
    return vec2<f32>(distance_marched,material);   
}


fn get_light(light_point: vec3<f32>,ray:Ray,surface_distance:f32,max_steps:i32,max_distance:f32) -> f32 {
    var light_position = vec3<f32>(0.0,2.0,0.0);
    let light = normalize(light_position-light_point);
    var light_ray = ray;
    light_ray.ro = light_point;
    let normal = get_normal(light_ray);
    
    var dif = clamp(dot(normal,light),0.0,1.0);

    return dif;
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let res = vec2<f32>(1280,720);
    var uv = vec2<f32>(f32(invocation_id.x), f32(invocation_id.y));
    uv = (uv-0.5*res)/res.y;
    let pi = 3.14159;

    var col = vec3<f32>(uv,1.0);
    let max_steps: i32 = 100;
    let max_distance: f32 = 100.0;
    let surface_distance: f32 = 0.01; 
    
    let ray_origin = camera.pos;
    var ray_direction = vec3(uv,-1.0);

    //fuck wgsl
    var rdt = ray_direction.yz;
    rdt *= rot_2d(camera.rot.y*pi);
    ray_direction = vec3<f32>(ray_direction.x,rdt.x,rdt.y);

    rdt = ray_direction.xz;
    rdt *= rot_2d(camera.rot.x*pi*2.0);
    ray_direction = vec3<f32>(rdt.x,ray_direction.y,rdt.y);
    
    var ray = Ray(ray_origin,ray_direction,1.0/ray_direction);

    let d = ray_march(&ray,max_steps,max_distance,surface_distance);
    let light_point = ray_origin + ray_direction * d.x;

    if d.x < max_distance {
	let diffuse_lighting = get_light(light_point,ray,surface_distance,max_steps,max_distance);
	col = vec3<f32>(diffuse_lighting);

	let material = i32(d.y);

	if (material == 0) {
	  col *= vec3(0.8,0.05,0.6);
	}
   }
    
    textureStore(texture, location, vec4(col,1.0));
}
