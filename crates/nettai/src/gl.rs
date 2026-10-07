//! The picture on femtovg's OpenGL: one texture, kept from picture to
//! picture and written in place, which the window draws as a borrowed
//! texture. (The retired nettai-demo's lesson: an image made anew each picture costs an
//! allocation, a conversion and a texture made and dropped each time.)
//!
//! The presented pixels go as they are: 0x00RRGGBB in memory is B, G, R and
//! an unused byte, which OpenGL takes as BGRA; the unused byte is made
//! opaque first. Everything here runs in the rendering notifier's
//! callbacks, where the window's context is current.

// (On the web the picture is a new image each time: WebGL's textures aren't
// borrowed by Slint there.)
#[cfg(target_arch = "wasm32")]
pub enum GlPicture {}

#[cfg(target_arch = "wasm32")]
impl GlPicture {
    pub fn write(&mut self, _: &mut [u32], _: u32, _: u32) -> Option<slint::Image> {
        match *self {}
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::GlPicture;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use glow::HasContext;
    use slint::{BorrowedOpenGLTextureBuilder, Image};
    use std::num::NonZeroU32;

    pub struct GlPicture {
        gl: glow::Context,
        texture: Option<(glow::Texture, u32, u32)>,
    }

    impl GlPicture {
        /// The window's OpenGL, through its loader.
        pub fn new(get_proc_address: &dyn Fn(&std::ffi::CStr) -> *const std::ffi::c_void) -> GlPicture {
            // SAFETY: the loader is the current context's, which stays current
            // in the callbacks this is used from.
            let gl = unsafe { glow::Context::from_loader_function_cstr(|name| get_proc_address(name)) };
            GlPicture { gl, texture: None }
        }

        /// Write `pixels` (0x00RRGGBB, `width` by `height`, made opaque here)
        /// into the texture; the image to show when the texture is made anew
        /// (the first picture, another size), else none: the image shown is
        /// already this texture.
        pub fn write(&mut self, pixels: &mut [u32], width: u32, height: u32) -> Option<Image> {
            for px in pixels.iter_mut() {
                *px |= 0xFF00_0000;
            }
            let gl = &self.gl;
            // SAFETY: the context is current; the texture is this context's, and
            // the slice holds `width * height` pixels.
            unsafe {
                let before = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D);
                let mut image = None;
                match self.texture {
                    Some((t, w, h)) if (w, h) == (width, height) => gl.bind_texture(glow::TEXTURE_2D, Some(t)),
                    old => {
                        if let Some((t, ..)) = old {
                            gl.delete_texture(t);
                        }
                        let t = gl.create_texture().ok()?;
                        gl.bind_texture(glow::TEXTURE_2D, Some(t));
                        gl.tex_image_2d(
                            glow::TEXTURE_2D,
                            0,
                            glow::RGBA8 as i32,
                            width as i32,
                            height as i32,
                            0,
                            glow::BGRA,
                            glow::UNSIGNED_INT_8_8_8_8_REV,
                            glow::PixelUnpackData::Slice(None),
                        );
                        for (param, value) in [
                            (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
                            (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
                            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
                        ] {
                            gl.tex_parameter_i32(glow::TEXTURE_2D, param, value as i32);
                        }
                        self.texture = Some((t, width, height));
                        let id = NonZeroU32::new(t.0.get())?;
                        image = Some(BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(id, (width, height).into()).build());
                    }
                }
                gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
                gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
                let bytes = std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), pixels.len() * 4);
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    glow::BGRA,
                    glow::UNSIGNED_INT_8_8_8_8_REV,
                    glow::PixelUnpackData::Slice(Some(bytes)),
                );
                gl.bind_texture(glow::TEXTURE_2D, NonZeroU32::new(before as u32).map(glow::NativeTexture));
                image
            }
        }
    }
}
