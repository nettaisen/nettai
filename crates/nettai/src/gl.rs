//! The picture on femtovg's OpenGL: one texture, kept from picture to
//! picture and written in place, which the window draws as a borrowed
//! texture. (The retired nettai-demo's lesson: an image made anew each picture costs an
//! allocation, a conversion and a texture made and dropped each time.)
//!
//! The presented pixels are 0x00RRGGBB; each is written over as 0xFFBBGGRR,
//! which in memory is R, G, B and an opaque A, and goes up as RGBA bytes:
//! the one format every OpenGL and OpenGL ES takes (BGRA and packed pixel
//! types aren't ES's, and a Windows driver that gives the window an ES or an
//! old context refused them: the picture stayed black). Should OpenGL still
//! refuse the texture (an error after writing it), the picture goes back to
//! being an image made anew each time ([`GlPicture::broken`]), and the log
//! says why. Everything here runs in the rendering notifier's callbacks,
//! where the window's context is current.

// (On the web the picture is a new image each time: WebGL's textures aren't
// borrowed by Slint there.)
#[cfg(target_arch = "wasm32")]
pub enum GlPicture {}

#[cfg(target_arch = "wasm32")]
impl GlPicture {
    pub fn broken(&self) -> bool {
        match *self {}
    }

    pub fn write(&mut self, _: &mut [u32], _: u32, _: u32) -> Result<Option<slint::Image>, String> {
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
        broken: bool,
    }

    impl GlPicture {
        /// The window's OpenGL, through its loader.
        pub fn new(get_proc_address: &dyn Fn(&std::ffi::CStr) -> *const std::ffi::c_void) -> GlPicture {
            // SAFETY: the loader is the current context's, which stays current
            // in the callbacks this is used from.
            let gl = unsafe { glow::Context::from_loader_function_cstr(|name| get_proc_address(name)) };
            GlPicture { gl, texture: None, broken: false }
        }

        /// OpenGL refused the texture once: the picture is an image from then on.
        pub fn broken(&self) -> bool {
            self.broken
        }

        /// Write `pixels` (0x00RRGGBB, `width` by `height`; turned into opaque
        /// RGBA bytes here) into the texture; the image to show when the texture
        /// is made anew (the first picture, another size), else none: the image
        /// shown is already this texture. An error is OpenGL's refusal, which
        /// leaves this broken.
        pub fn write(&mut self, pixels: &mut [u32], width: u32, height: u32) -> Result<Option<Image>, String> {
            assert_eq!(pixels.len(), width as usize * height as usize, "a picture's pixels are its width by its height");
            // (Every target nettai runs on is little-endian: 0xFFBBGGRR is R, G, B, A in memory.)
            swap_red_blue(pixels);
            let (gl, texture) = (&self.gl, &mut self.texture);
            // SAFETY: the context is current; the texture is this context's, and
            // the slice holds `width * height` pixels (asserted).
            let result = (|| unsafe {
                // (Errors from before, the renderer's, aren't this picture's.)
                while gl.get_error() != glow::NO_ERROR {}
                let before = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D);
                let mut image = None;
                match *texture {
                    Some((t, w, h)) if (w, h) == (width, height) => gl.bind_texture(glow::TEXTURE_2D, Some(t)),
                    old => {
                        if let Some((t, ..)) = old {
                            gl.delete_texture(t);
                        }
                        let t = gl.create_texture().map_err(|e| format!("no texture: {e}"))?;
                        gl.bind_texture(glow::TEXTURE_2D, Some(t));
                        gl.tex_image_2d(
                            glow::TEXTURE_2D,
                            0,
                            glow::RGBA as i32,
                            width as i32,
                            height as i32,
                            0,
                            glow::RGBA,
                            glow::UNSIGNED_BYTE,
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
                        *texture = Some((t, width, height));
                        let id = NonZeroU32::new(t.0.get()).ok_or("a texture named 0")?;
                        image = Some(BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(id, (width, height).into()).build());
                    }
                }
                gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
                // (ES 2 has no row length: there it is always the width.)
                let row_length = !gl.version().is_embedded || gl.version().major >= 3;
                if row_length {
                    gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
                }
                let bytes = std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), pixels.len() * 4);
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(bytes)),
                );
                let error = gl.get_error();
                gl.bind_texture(glow::TEXTURE_2D, NonZeroU32::new(before as u32).map(glow::NativeTexture));
                match error {
                    glow::NO_ERROR => Ok(image),
                    e => {
                        let v = gl.version();
                        Err(format!("OpenGL{} {}.{} ({}) refused the picture's texture: error {e:#x}", if v.is_embedded { " ES" } else { "" }, v.major, v.minor, v.vendor_info))
                    }
                }
            })();
            if result.is_err() {
                // (The pixels as they came, for the image made in its place.)
                swap_red_blue(pixels);
                self.broken = true;
            }
            result
        }
    }

    /// 0x00RRGGBB as opaque 0xFFBBGGRR, and back (the swap is its own undoing; the top byte stays opaque).
    fn swap_red_blue(pixels: &mut [u32]) {
        for px in pixels.iter_mut() {
            let p = *px;
            *px = 0xFF00_0000 | (p & 0xFF) << 16 | (p & 0xFF00) | (p >> 16 & 0xFF);
        }
    }
}
