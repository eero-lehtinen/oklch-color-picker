use std::{cell::OnceCell, num::NonZeroU64, sync::OnceLock};

use bevy_color::ColorToComponents as _;
use bytemuck::{Pod, Zeroable};
use eframe::{
    egui_wgpu::wgpu::util::DeviceExt as _,
    egui_wgpu::{self, wgpu},
};
use egui::{Vec2, ahash::HashMap};
use strum::{IntoDiscriminant, IntoEnumIterator as _};
use wgpu::ColorTargetState;

use crate::app::{CurrentColors, CurrentColorsDiscriminants, Fallbacks};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, strum::Display, strum::EnumIter)]
pub enum RenderKind {
    Picker1,
    Picker2,
    Slider1,
    Slider2,
    Slider3,
    Slider4,
    Final,
}

static SUPERSAMPLE: OnceLock<bool> = OnceLock::new();

pub fn init(cc: &eframe::CreationContext) -> Option<()> {
    // Get the WGPU render state from the eframe creation context. This can also be retrieved
    // from `eframe::Frame` when you don't have a `CreationContext` available.
    let wgpu_render_state = cc.wgpu_render_state.as_ref()?;

    let device = &wgpu_render_state.device;

    SUPERSAMPLE
        .set(
            !cc.egui_ctx
                .native_pixels_per_point()
                .is_some_and(|p| p > 2.1),
        )
        .unwrap();

    let resources = RenderKind::iter().map(|kind| {
        let frag_shader_source = match kind {
            RenderKind::Picker1 => include_str!("shaders/fragment.wgsl"),
            RenderKind::Picker2 => include_str!("shaders/fragment.wgsl"),
            RenderKind::Slider1 => include_str!("shaders/fragment.wgsl"),
            RenderKind::Slider2 => include_str!("shaders/fragment.wgsl"),
            RenderKind::Slider3 => include_str!("shaders/fragment.wgsl"),
            RenderKind::Slider4 => include_str!("shaders/fragment.wgsl"),
            RenderKind::Final => include_str!("shaders/fragment.wgsl"),
        };

        let label = kind.to_string();

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&label),
            source: wgpu::ShaderSource::Wgsl(
                [include_str!("shaders/shared.wgsl"), frag_shader_source]
                    .concat()
                    .into(),
            ),
        });

        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&label),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("shaders/shared.wgsl"),
                    include_str!("shaders/vertex.wgsl")
                )
                .into(),
            ),
        });

        macro_rules! layout_entry {
            ($binding:expr) => {
                wgpu::BindGroupLayoutEntry {
                    binding: $binding,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            };
        }

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&label),
            entries: &[
                layout_entry!(0),
                layout_entry!(1),
                layout_entry!(2),
                layout_entry!(3),
                layout_entry!(4),
                layout_entry!(5),
                layout_entry!(6),
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&label),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&label),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: Some("main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: Some("main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu_render_state.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        macro_rules! create_buffer_init {
            ($data:expr) => {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&label),
                    contents: bytemuck::bytes_of(&$data),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                })
            };
        }

        let prev_color_buffer = create_buffer_init!([0.; 4]);
        let color_buffer = create_buffer_init!([0.; 4]);
        let values_buffer = create_buffer_init!([0.; 3]);
        let size_buffer = create_buffer_init!([0.; 2]);
        let kind_buffer = create_buffer_init!([0u32]);
        let mode_buffer = create_buffer_init!([0u32]);
        let supersample_buffer = create_buffer_init!([0u32]);

        macro_rules! entry {
            ($buffer:expr, $binding:expr) => {
                wgpu::BindGroupEntry {
                    binding: $binding,
                    resource: $buffer.as_entire_binding(),
                }
            };
        }

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&label),
            layout: &bind_group_layout,
            entries: &[
                entry!(prev_color_buffer, 0),
                entry!(color_buffer, 1),
                entry!(values_buffer, 2),
                entry!(size_buffer, 3),
                entry!(kind_buffer, 4),
                entry!(mode_buffer, 5),
                entry!(supersample_buffer, 6),
            ],
        });

        (
            kind,
            RenderResources {
                pipeline,
                bind_group,
                prev_color_buffer,
                color_buffer,
                values_buffer,
                size_buffer,
                kind_buffer,
                mode_buffer,
                supersample_buffer,
            },
        )
    });

    // Because the graphics pipeline must have the same lifetime as the egui render pass,
    // instead of storing the pipeline in our `Custom3D` struct, we insert it into the
    // `paint_callback_resources` type map, which is stored alongside the render pass.
    wgpu_render_state
        .renderer
        .write()
        .callback_resources
        .insert(RenderResourcesMap(resources.collect()))?;

    Some(())
}

struct Uniforms {
    prev_color: [f32; 4],
    color: [f32; 4],
    values: [f32; 3],
    size: [f32; 2],
    kind: u32,
    mode: u32,
    supersample: u32,
}

// Callbacks in egui_wgpu have 3 stages:
// * prepare (per callback impl)
// * finish_prepare (once)
// * paint (per callback impl)
//
// The prepare callback is called every frame before paint and is given access to the wgpu
// Device and Queue, which can be used, for instance, to update buffers and uniforms before
// rendering.
// If [`egui_wgpu::Renderer`] has [`egui_wgpu::FinishPrepareCallback`] registered,
// it will be called after all `prepare` callbacks have been called.
// You can use this to update any shared resources that need to be updated once per frame
// after all callbacks have been processed.
//
// On both prepare methods you can use the main `CommandEncoder` that is passed-in,
// return an arbitrary number of user-defined `CommandBuffer`s, or both.
// The main command buffer, as well as all user-defined ones, will be submitted together
// to the GPU in a single call.
//
// The paint callback is called after finish prepare and is given access to egui's main render pass,
// which can be used to issue draw commands.
pub struct RenderCallback {
    pub kind: RenderKind,
    pub size: Vec2,
    pub colors: CurrentColors,
    pub fallbacks: Fallbacks,
}

impl egui_wgpu::CallbackTrait for RenderCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let resources: &RenderResourcesMap = resources.get().unwrap();
        resources.0[&self.kind].prepare(
            device,
            queue,
            Uniforms {
                prev_color: self.fallbacks.prev.to_f32_array(),
                color: self.fallbacks.cur.to_f32_array(),
                values: self.colors.values()[0..3].try_into().unwrap(),
                size: self.size.into(),
                kind: (self.kind as u32),
                mode: self.colors.discriminant() as u32,
                supersample: *SUPERSAMPLE.get().unwrap() as u32,
            },
        );
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let resources: &RenderResourcesMap = resources.get().unwrap();
        resources.0[&self.kind].paint(render_pass);
    }
}

pub fn paint(ui: &mut egui::Ui, rect: egui::Rect, render_callback: RenderCallback) {
    ui.painter().add(egui_wgpu::Callback::new_paint_callback(
        rect,
        render_callback,
    ));
}

struct RenderResourcesMap(HashMap<RenderKind, RenderResources>);

struct RenderResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    prev_color_buffer: wgpu::Buffer,
    color_buffer: wgpu::Buffer,
    values_buffer: wgpu::Buffer,
    size_buffer: wgpu::Buffer,
    kind_buffer: wgpu::Buffer,
    mode_buffer: wgpu::Buffer,
    supersample_buffer: wgpu::Buffer,
}

impl RenderResources {
    fn prepare(&self, _device: &wgpu::Device, queue: &wgpu::Queue, uniforms: Uniforms) {
        queue.write_buffer(
            &self.prev_color_buffer,
            0,
            bytemuck::bytes_of(&uniforms.prev_color),
        );
        queue.write_buffer(&self.color_buffer, 0, bytemuck::bytes_of(&uniforms.color));
        queue.write_buffer(&self.values_buffer, 0, bytemuck::bytes_of(&uniforms.values));
        queue.write_buffer(&self.size_buffer, 0, bytemuck::bytes_of(&uniforms.size));
        queue.write_buffer(&self.kind_buffer, 0, bytemuck::bytes_of(&uniforms.kind));
        queue.write_buffer(&self.mode_buffer, 0, bytemuck::bytes_of(&uniforms.mode));
        queue.write_buffer(
            &self.supersample_buffer,
            0,
            bytemuck::bytes_of(&uniforms.supersample),
        );
    }

    fn paint(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
