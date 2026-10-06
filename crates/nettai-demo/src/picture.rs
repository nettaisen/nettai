//! The battle's picture in the window: what the player presented, at the
//! window's size, shown over the whole of it.
//!
//! On iced's GPU renderer it is one texture, kept from frame to frame and
//! written in place (`queue.write_texture`) when the picture changes, drawn
//! scaled with the nearest texel. (An image widget given a new
//! `image::Handle` each tick flickered: iced uploads a raster image of 2 MiB
//! or more, the window's picture at its default size, on a thread of its
//! own, and draws nothing for it until that is done, so the frames on which
//! a new picture first came showed none.) On the software renderer, the
//! fallback, it is an image, which that renderer draws at once.

use iced::advanced::image::{self as raster, Renderer as _};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Tree, Widget};
use iced::widget::shader::{self, Viewport};
use iced::{Element, Length, Rectangle, Size, mouse, wgpu};
use iced_wgpu::primitive::Renderer as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// A picture the player presented: its pixels (0x00RRGGBB, a row after
/// another) and its size.
pub struct Picture {
    /// Which picture it is (a texture holding it needn't be written again).
    serial: u64,
    width: u32,
    height: u32,
    pixels: Vec<u32>,
    /// As an image, for the software renderer (made the first time it draws
    /// the picture).
    image: OnceLock<raster::Handle>,
    /// When the player presented it.
    made: Instant,
    /// When the first key pressed that a tick of it saw first came, if any.
    key: Option<Instant>,
}

/// Times, for `NETTAI_PLAY_STATS`: how many, their total and the longest.
#[derive(Clone, Copy, Default)]
pub struct Times {
    pub count: u32,
    pub total: Duration,
    pub worst: Duration,
}

impl Times {
    pub fn add(&mut self, t: Duration) {
        self.count += 1;
        self.total += t;
        self.worst = self.worst.max(t);
    }

    pub fn mean(&self) -> Duration {
        self.total / self.count.max(1)
    }
}

/// When the pictures were first drawn (handed to the GPU, or drawn by the
/// software renderer): how long after the player presented them, and after
/// the key a tick of theirs saw first came; since [`take_shown`] last took
/// them.
static SHOWN: Mutex<(Times, Times)> = Mutex::new((Times { count: 0, total: Duration::ZERO, worst: Duration::ZERO }, Times { count: 0, total: Duration::ZERO, worst: Duration::ZERO }));

/// The serial of the picture the software renderer drew last.
static DRAWN: AtomicU64 = AtomicU64::new(u64::MAX);

fn shown(picture: &Picture) {
    let mut s = SHOWN.lock().unwrap_or_else(|e| e.into_inner());
    s.0.add(picture.made.elapsed());
    if let Some(key) = picture.key {
        s.1.add(key.elapsed());
    }
}

/// The pictures first drawn since the last call: how long after they were
/// presented, and after the keys their ticks saw first came.
pub fn take_shown() -> (Times, Times) {
    std::mem::take(&mut *SHOWN.lock().unwrap_or_else(|e| e.into_inner()))
}

impl Picture {
    /// `pixels` at `width` by `height`; `key`: when the first key pressed
    /// that a tick of it saw first came.
    pub fn new(width: usize, height: usize, pixels: Vec<u32>, key: Option<Instant>) -> Picture {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        assert_eq!(pixels.len(), width * height);
        Picture {
            serial: SERIAL.fetch_add(1, Ordering::Relaxed),
            width: width as u32,
            height: height as u32,
            pixels,
            image: OnceLock::new(),
            made: Instant::now(),
            key,
        }
    }

    fn image(&self) -> &raster::Handle {
        self.image.get_or_init(|| {
            let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
            for &px in &self.pixels {
                rgba.extend_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, px as u8, 0xFF]);
            }
            raster::Handle::from_rgba(self.width, self.height, rgba)
        })
    }
}

/// `picture` over the whole of the space it is given.
pub fn view<'a, Message: 'a>(picture: &Arc<Picture>) -> Element<'a, Message> {
    Element::new(Shown { picture: picture.clone() })
}

struct Shown {
    picture: Arc<Picture>,
}

impl<Message, Theme> Widget<Message, Theme, iced::Renderer> for Shown {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, _: &mut Tree, _: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        layout::atomic(limits, Length::Fill, Length::Fill)
    }

    fn draw(&self, _: &Tree, renderer: &mut iced::Renderer, _: &Theme, _: &renderer::Style, layout: Layout<'_>, _: mouse::Cursor, _: &Rectangle) {
        let bounds = layout.bounds();
        if matches!(renderer, iced::Renderer::Primary(_)) {
            renderer.draw_primitive(bounds, Primitive(self.picture.clone()));
        } else {
            let image = raster::Image::new(self.picture.image().clone()).filter_method(raster::FilterMethod::Nearest);
            renderer.draw_image(image, bounds, bounds);
            if DRAWN.swap(self.picture.serial, Ordering::Relaxed) != self.picture.serial {
                shown(&self.picture);
            }
        }
    }
}

/// The picture, for the GPU renderer.
struct Primitive(Arc<Picture>);

impl std::fmt::Debug for Primitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "picture {} ({}x{})", self.0.serial, self.0.width, self.0.height)
    }
}

/// The GPU's side: the texture the picture is in, and what draws it.
struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// The texture's format: the window's pixels as they are, in a format
    /// whose sRGB-ness is the window's own, so what is drawn is what was
    /// presented.
    format: wgpu::TextureFormat,
    texture: Option<Texture>,
}

struct Texture {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    size: (u32, u32),
    /// The picture in it.
    serial: u64,
}

/// A triangle over the whole viewport (the picture's bounds), sampling the
/// texture across them.
const SHADER: &str = "
@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var nearest: sampler;

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) i: u32) -> Out {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: Out;
    out.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(picture, nearest, in.uv).rgb, 1.0);
}
";

impl shader::Pipeline for Pipeline {
    fn new(device: &wgpu::Device, _: &wgpu::Queue, target: wgpu::TextureFormat) -> Pipeline {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nettai picture"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nettai picture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nettai picture"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nettai picture"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: target, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nettai picture"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        // (0x00RRGGBB in memory, little-endian: B, G, R and an unused byte.)
        let format = if target.is_srgb() { wgpu::TextureFormat::Bgra8UnormSrgb } else { wgpu::TextureFormat::Bgra8Unorm };
        Pipeline { pipeline, layout, sampler, format, texture: None }
    }
}

impl shader::Primitive for Primitive {
    type Pipeline = Pipeline;

    fn prepare(&self, p: &mut Pipeline, device: &wgpu::Device, queue: &wgpu::Queue, _: &Rectangle, _: &Viewport) {
        let picture = &self.0;
        let size = (picture.width, picture.height);
        if p.texture.as_ref().is_none_or(|t| t.size != size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("nettai picture"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: p.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("nettai picture"),
                layout: &p.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&p.sampler) },
                ],
            });
            p.texture = Some(Texture { texture, bind_group, size, serial: u64::MAX });
        }
        let t = p.texture.as_mut().expect("made above");
        if t.serial != picture.serial {
            queue.write_texture(
                t.texture.as_image_copy(),
                bytemuck::cast_slice(&picture.pixels),
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * size.0), rows_per_image: Some(size.1) },
                wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            );
            t.serial = picture.serial;
            shown(picture);
        }
    }

    fn draw(&self, p: &Pipeline, pass: &mut wgpu::RenderPass<'_>) -> bool {
        let Some(t) = &p.texture else { return true };
        pass.set_pipeline(&p.pipeline);
        pass.set_bind_group(0, &t.bind_group, &[]);
        pass.draw(0..3, 0..1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::shader::{Pipeline as _, Primitive as _};

    /// The texture drawn over a target twice its size, as on a Retina
    /// display: each of the picture's pixels four times, its color as it
    /// was, on a target of either kind (sRGB or not). Off screen, on any
    /// GPU adapter there is (none: skipped).
    #[test]
    fn the_picture_is_drawn_as_presented() {
        use iced::futures::executor::block_on;
        let instance = wgpu::Instance::default();
        let Ok(adapter) = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            eprintln!("no GPU adapter: skipped");
            return;
        };
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let (w, h) = (3usize, 2usize);
        let pixels = vec![0x00FF0000, 0x0000FF00, 0x000000FF, 0x00123456, 0x00FFFFFF, 0x00808080];
        let primitive = Primitive(Arc::new(Picture::new(w, h, pixels.clone(), None)));
        for target in [wgpu::TextureFormat::Bgra8Unorm, wgpu::TextureFormat::Bgra8UnormSrgb] {
            let mut pipeline = Pipeline::new(&device, &queue, target);
            let viewport = Viewport::with_physical_size(Size::new(2 * w as u32, 2 * h as u32), 1.0);
            primitive.prepare(&mut pipeline, &device, &queue, &Rectangle::default(), &viewport);
            let out = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d { width: 2 * w as u32, height: 2 * h as u32, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: target,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = out.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                assert!(primitive.draw(&pipeline, &mut pass));
            }
            // (A row of a copy to a buffer is a multiple of 256 bytes.)
            let row = 256u32;
            let read = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (row * 2 * h as u32) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                out.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read,
                    layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(2 * h as u32) },
                },
                wgpu::Extent3d { width: 2 * w as u32, height: 2 * h as u32, depth_or_array_layers: 1 },
            );
            queue.submit([encoder.finish()]);
            read.slice(..).map_async(wgpu::MapMode::Read, |r| r.unwrap());
            device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
            let bytes = read.slice(..).get_mapped_range().to_vec();
            for y in 0..2 * h {
                for x in 0..2 * w {
                    let at = y * row as usize + 4 * x;
                    let [b, g, r, a] = [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]];
                    let shown = (r as u32) << 16 | (g as u32) << 8 | b as u32;
                    assert_eq!((shown, a), (pixels[y / 2 * w + x / 2], 0xFF), "{target:?}: the pixel at {x},{y}");
                }
            }
        }
    }
}
