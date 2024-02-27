use std::sync::Arc;

use glam::{Quat, Vec3};
use image::{ImageBuffer, Rgba};

use gobs::{
    core::{
        entity::{camera::Camera, light::Light},
        Color, Transform,
    },
    game::{
        app::{Application, Run},
        input::{Input, Key},
    },
    render::{
        context::Context,
        geometry::{Model, VertexFlag},
        graph::{FrameGraph, RenderError},
        material::{Material, MaterialProperty, Texture, TextureType},
        pass::PassType,
        renderable::Renderable,
        SamplerFilter,
    },
    scene::{graph::scenegraph::NodeValue, scene::Scene, shape::Shapes},
};

use raytracer::raytracer::{ChunkStrategy, Ray, Sphere, Tracer, TracerBuilder};

struct App {
    pub graph: FrameGraph,
    pub scene: Scene,
    tracer: Tracer,
    material: Arc<Material>,
}

impl Run for App {
    async fn create(ctx: &Context) -> Self {
        log::info!("Create");

        let graph = FrameGraph::default(ctx);

        let light = Light::new((0., 0., 10.), Color::WHITE);

        let extent = ctx.surface.get_extent(ctx.device.clone());

        let camera = Camera::ortho(
            (0., 0., 1.),
            extent.width as f32,
            extent.height as f32,
            0.1,
            100.,
            0.,
            0.,
            Vec3::Y,
        );

        let scene = Scene::new(camera, light);

        let tracer = TracerBuilder::new(extent)
            .await
            .camera(Camera::perspective(
                Vec3::new(0., 0.2, 0.),
                extent.width as f32 / extent.height as f32,
                (45. as f32).to_radians(),
                0.1,
                100.,
                (-90. as f32).to_radians(),
                (0. as f32).to_radians(),
                Vec3::Y,
            ))
            .rays(10)
            .reflects(10)
            .threads(8)
            .light(Light::new(Vec3::new(0., 2., -2.), Color::WHITE))
            .model(Sphere::new(
                "ground",
                Vec3::new(0., -5000.2, 0.),
                5000.,
                Color::GREY,
                0.1,
            ))
            .model(Sphere::new(
                "black",
                Vec3::new(0., 0.5, 1.2),
                0.3,
                Color::BLACK,
                0.8,
            ))
            .model(Sphere::new(
                "green",
                Vec3::new(-0.5, 0.2, 0.7),
                0.3,
                Color::GREEN,
                0.4,
            ))
            .model(Sphere::new(
                "red",
                Vec3::new(0.5, 0.2, 0.7),
                0.3,
                Color::RED,
                0.25,
            ))
            .background(Self::background_color)
            .strategy(ChunkStrategy::BOX)
            .build()
            .await;

        let vertex_flags = VertexFlag::POSITION
            | VertexFlag::TEXTURE
            | VertexFlag::NORMAL
            | VertexFlag::TANGENT
            | VertexFlag::BITANGENT;

        let material = Material::builder("mesh.vert.spv", "mesh.frag.spv")
            .vertex_flags(vertex_flags)
            .prop("diffuse", MaterialProperty::Texture)
            .build(ctx, graph.pass_by_type(PassType::Forward).unwrap());

        App {
            graph,
            scene,
            tracer,
            material,
        }
    }

    fn update(&mut self, ctx: &Context, delta: f32) {
        if self.tracer.update() {
            let framebuffer = self.tracer.framebuffer();

            let extent = self.tracer.extent();

            let texture = Texture::with_colors(
                ctx,
                framebuffer,
                extent,
                TextureType::Diffuse,
                SamplerFilter::FilterLinear,
            );

            let material_instance = self.material.instantiate(vec![texture]);

            let rect = Model::builder("rect")
                .mesh(Shapes::quad(), material_instance)
                .build();

            let transform = Transform::new(
                [0., 0., 0.].into(),
                Quat::IDENTITY,
                [extent.width as f32, extent.height as f32, 1.].into(),
            );

            let root = self.scene.graph.get(self.scene.graph.root).unwrap();

            for child in root.children.clone() {
                self.scene.graph.remove(child);
            }

            self.scene
                .graph
                .insert(self.scene.graph.root, NodeValue::Model(rect), transform);
        }

        self.scene.update(ctx, delta);
    }

    fn render(&mut self, ctx: &Context) -> Result<(), RenderError> {
        log::trace!("Render frame {}", self.graph.frame_number);

        self.graph.begin(ctx)?;

        self.graph.render(ctx, &mut |pass, batch| match pass.ty() {
            PassType::Compute => {}
            PassType::Depth => {
                self.scene.draw(ctx, pass, batch);
            }
            PassType::Forward => {
                self.scene.draw(ctx, pass, batch);
            }
            PassType::Wire => {}
            PassType::Ui => {}
        })?;

        self.graph.end(ctx)?;

        log::trace!("End render");

        Ok(())
    }

    fn resize(&mut self, ctx: &Context, width: u32, height: u32) {
        log::trace!("Resize");

        self.graph.resize(ctx);
        self.scene.resize(width, height);
    }

    fn input(&mut self, _ctx: &Context, input: Input) {
        match input {
            Input::KeyPressed(key) => match key {
                Key::P => self.screenshot(),
                _ => (),
            },
            _ => (),
        }
    }

    async fn start(&mut self, _ctx: &Context) {}

    fn close(&mut self, ctx: &Context) {
        log::info!("Closing");

        ctx.device.wait();

        log::info!("Closed");
    }
}

impl App {
    fn background_color(ray: &Ray) -> Color {
        let dot_x = ray.direction.dot(Vec3::X);
        let dot_y = ray.direction.dot(Vec3::Y);

        Color::new(0.2 * dot_x, 0.5 + 0.5 * dot_y, 1., 1.)
    }

    fn screenshot(&self) {
        let buffer = self.tracer.bytes();
        let file_name = "raytracer.png";

        let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(
            self.tracer.extent().width,
            self.tracer.extent().height,
            buffer,
        )
        .unwrap();

        img.save(file_name).expect("Saving");

        log::info!("Image save: {}", file_name);
    }
}

fn main() {
    raytracer::init_logger();

    Application::new("Raytracer", 1920, 1080).run::<App>();
}
