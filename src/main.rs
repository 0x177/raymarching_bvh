use bevy::{
    prelude::*,
    render::{
	texture::{FallbackImage},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssetUsages,
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        Render, RenderApp, RenderSet,
    },
    window::WindowPlugin,
    diagnostic::{FrameTimeDiagnosticsPlugin},
};

mod bvh;
use bvh::*;
mod bunny;
use bunny::*;

use std::borrow::Cow;

//use rand::prelude::*;

use iyes_perf_ui::prelude::*;

const SIZE: (u32, u32) = (1920, 1080);
const WORKGROUP_SIZE: u32 = 8;

#[derive(Resource)]
struct RayMarcherPipeline {
    texture_bind_group_layout: BindGroupLayout,
    bvh_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

#[derive(Resource, Clone, Deref, ExtractResource, AsBindGroup)]
struct RayMarcherImage {
    #[storage_texture(0, image_format = Rgba8Unorm, access = ReadWrite)]
    texture: Handle<Image>,
}

#[derive(Resource)]
struct RayMarcherImageBindGroup(BindGroup);

impl FromWorld for RayMarcherPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
	
        let texture_bind_group_layout = RayMarcherImage::bind_group_layout(render_device);

	let bvh_bind_group_layout = render_device.create_bind_group_layout(
	    Some("bvh bind group layout"),
	    &[
		BindGroupLayoutEntry {
		    binding: 0,
		    visibility: ShaderStages::COMPUTE,
		    ty: BindingType::Buffer {
			ty: BufferBindingType::Storage{read_only: true},
			has_dynamic_offset: false,
			min_binding_size: None,
		    },
		    count: None
		},
		BindGroupLayoutEntry {
		    binding: 1,
		    visibility: ShaderStages::COMPUTE,
		    ty: BindingType::Buffer {
			ty: BufferBindingType::Storage{read_only: true},
			has_dynamic_offset: false,
			min_binding_size: None,
		    },
		    count: None
		},
		BindGroupLayoutEntry {
		    binding: 2,
		    visibility: ShaderStages::COMPUTE,
		    ty: BindingType::Buffer {
			ty: BufferBindingType::Uniform,
			has_dynamic_offset: false,
			min_binding_size: None,
		    },
		    count: None
		}
	    ]
	);
	
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/raymarcher.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
	
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("init pipeline")),
            layout: vec![texture_bind_group_layout.clone(),bvh_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
        });
	
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label:  Some(Cow::from("update pipeline")),
            layout: vec![texture_bind_group_layout.clone(),bvh_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        RayMarcherPipeline {
            texture_bind_group_layout,
	    bvh_bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

enum RayMarcherState {
    Loading,
    Init,
    Update,
}

struct RayMarcherNode {
    state: RayMarcherState,
}

impl Default for RayMarcherNode {
    fn default() -> Self {
        Self {
            state: RayMarcherState::Loading,
        }
    }
}

struct RayMarcherComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct RayMarcherLabel;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    // uncomment for unthrottled FPS
                    present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            RayMarcherComputePlugin,
	    FrameTimeDiagnosticsPlugin::default(),
	    PerfUiPlugin
        ))
        .add_systems(Startup, setup)
	.add_systems(Update,data_update)
        .run();
}

fn data_update(mut query: Query<&mut RayMarcherData>) {
    for mut data in &mut query {
	data.camera.pos.z += 0.5;
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn(PerfUiCompleteBundle::default());
    // this place is reached
    let mut image = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let image = images.add(image);

    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(SIZE.0 as f32, SIZE.1 as f32)),
            ..default()
        },
        texture: image.clone(),
        ..default()
    });
    commands.spawn(Camera2dBundle::default());

    //let mut rng = thread_rng();
    
    let mut scene = vec![];
    
    let mut bvh = vec![
	bvh::Node::new(Vec3::new(0.0,0.0,0.0),Vec3::new(0.0,0.0,0.0),0,0),
    ];

    for i in 0..100 {
	let x = i%10;
	let y = i/10;

	let v = Vec3::new(x as f32*0.1,y as f32*0.1,-7.0);
	scene.push(
	    Object {
		pos: v,
		ty: 0.0,
		params: [0.1,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0]
	    }
	);
	bvh[0].grow_to_include_object(&scene[i]);
    }

    let camera: BvhCamera = BvhCamera {
	pos: Vec3::new(0.0,0.0,-5.0),
	rot: Vec2::new(0.0,0.0)
    };
    
    //please god let this be the last hack in this project
    let mut data =bvh::RayMarcherData {bvh,scene,camera};
    let mut temp = data.clone();
    temp.split(&mut data.bvh[0],1);
    data = temp;
    
    commands.insert_resource(RayMarcherImage { texture: image });

    commands.insert_resource(data);
}

impl Plugin for RayMarcherComputePlugin {
    fn build(&self, app: &mut App) {
	// this place is reached
        app.add_plugins(ExtractResourcePlugin::<RayMarcherImage>::default());
	app.add_plugins(ExtractResourcePlugin::<RayMarcherData>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_image_bind_group.in_set(RenderSet::PrepareBindGroups),
        );
	
	render_app.add_systems(
            Render,
            prepare_bvh_bind_group.in_set(RenderSet::Prepare),
        );

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(RayMarcherLabel, RayMarcherNode::default());
        render_graph.add_node_edge(RayMarcherLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<RayMarcherPipeline>();
    }
}

fn prepare_image_bind_group(
    mut commands: Commands,
    pipeline: Res<RayMarcherPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    image: Res<RayMarcherImage>,
    render_device: Res<RenderDevice>,
) {
    let view = gpu_images.get(&image.texture).unwrap();
    let bind_group = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::single(&view.texture_view),
    );
    commands.insert_resource(RayMarcherImageBindGroup(bind_group));
}

fn prepare_bvh_bind_group(
    mut commands: Commands,
    data: Res<RayMarcherData>,
    render_device: Res<RenderDevice>,
    pipeline: Res<RayMarcherPipeline>,
    fallback_image: Res<FallbackImage>,
    images: Res<RenderAssets<Image>>,
) {
    let prepared_result = data.as_bind_group(
	&pipeline.bvh_bind_group_layout,
	&render_device,
	&images,
	&fallback_image
    ).unwrap();

    commands.insert_resource(RayMarcherBVHBindGroup(prepared_result.bind_group));
}

impl render_graph::Node for RayMarcherNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<RayMarcherPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            RayMarcherState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = RayMarcherState::Init;
                }
            }
            RayMarcherState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = RayMarcherState::Update;
                }
            }
            RayMarcherState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<RayMarcherImageBindGroup>().0;
	let bvh_bind_group = &world.resource::<RayMarcherBVHBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<RayMarcherPipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);
	pass.set_bind_group(1, bvh_bind_group, &[]);

        match self.state {
            RayMarcherState::Loading => {}
            RayMarcherState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
            RayMarcherState::Update => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
        }

        Ok(())
    }
}
