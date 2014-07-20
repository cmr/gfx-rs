// Copyright 2014 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![feature(globs)]
#![crate_name = "gldevice"]

extern crate gl;
extern crate libc;
extern crate device;

use log;
use std;
use a = device::attrib;
use std::fmt;
use std::str;
use std::collections::HashSet;

mod rast;
mod shade;
mod tex;

pub type Buffer         = gl::types::GLuint;
pub type ArrayBuffer    = gl::types::GLuint;
pub type Shader         = gl::types::GLuint;
pub type Program        = gl::types::GLuint;
pub type FrameBuffer    = gl::types::GLuint;
pub type Surface        = gl::types::GLuint;
pub type Sampler        = gl::types::GLuint;
#[deriving(Eq, Ord, PartialEq, PartialOrd, Hash, Clone, Show)]
pub struct Texture {
    name: gl::types::GLuint,
    kind: gl::types::GLenum
}

fn get_uint(name: gl::types::GLenum) -> uint {
    let mut value = 0 as gl::types::GLint;
    unsafe { gl::GetIntegerv(name, &mut value) };
    value as uint
}

/// Get a statically allocated string from the implementation using
/// `glGetString`. Fails if it `GLenum` cannot be handled by the
/// implementation's `gl::GetString` function.
fn get_string(name: gl::types::GLenum) -> &'static str {
    let ptr = gl::GetString(name) as *const i8;
    if !ptr.is_null() {
        // This should be safe to mark as statically allocated because
        // GlGetString only returns static strings.
        unsafe { str::raw::c_str_to_static_slice(ptr) }
    } else {
        fail!("Invalid GLenum passed to `get_string`: {:x}", name)
    }
}

pub type VersionMajor = uint;
pub type VersionMinor = uint;
pub type Revision = uint;
pub type VendorDetails = &'static str;

/// A version number for a specific component of an OpenGL implementation
#[deriving(Eq, PartialEq, Ord, PartialOrd)]
pub struct Version(VersionMajor, VersionMinor, Option<Revision>, VendorDetails);

impl Version {
    /// According to the OpenGL spec, the version information is expected to
    /// follow the following syntax:
    ///
    /// ~~~bnf
    /// <major>       ::= <number>
    /// <minor>       ::= <number>
    /// <revision>    ::= <number>
    /// <vendor-info> ::= <string>
    /// <release>     ::= <major> "." <minor> ["." <release>]
    /// <version>     ::= <release> [" " <vendor-info>]
    /// ~~~
    ///
    /// Note that this function is intentionally lenient in regards to parsing,
    /// and will try to recover at least the first two version numbers without
    /// resulting in an `Err`.
    fn parse(src: &'static str) -> Result<Version, &'static str> {
        let (version, vendor_info) = match src.find(' ') {
            Some(i) => (src.slice_to(i), src.slice_from(i + 1)),
            None => (src, ""),
        };

        // TODO: make this even more lenient so that we can also accept
        // `<major> "." <minor> [<???>]`
        let mut it = version.split('.');
        let major = it.next().and_then(from_str);
        let minor = it.next().and_then(from_str);
        let revision = it.next().and_then(from_str);

        match (major, minor, revision) {
            (Some(major), Some(minor), revision) =>
                Ok(Version(major, minor, revision, vendor_info)),
            (_, _, _) => Err(src),
        }
    }
}

impl fmt::Show for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Version(major, minor, Some(revision), "") =>
                write!(f, "Version({}.{}.{})", major, minor, revision),
            Version(major, minor, None, "") =>
                write!(f, "Version({}.{})", major, minor),
            Version(major, minor, Some(revision), vendor_info) =>
                write!(f, "Version({}.{}.{}, {})", major, minor, revision, vendor_info),
            Version(major, minor, None, vendor_info) =>
                write!(f, "Version({}.{}, {})", major, minor, vendor_info),
        }
    }
}

/// A unique platform identifier that does not change between releases
#[deriving(Eq, PartialEq, Show)]
pub struct PlatformName {
    /// The company responsible for the OpenGL implementation
    pub vendor: &'static str,
    /// The name of the renderer
    pub renderer: &'static str,
}

impl PlatformName {
    fn get() -> PlatformName {
        PlatformName {
            vendor: get_string(gl::VENDOR),
            renderer: get_string(gl::RENDERER),
        }
    }
}

/// OpenGL implementation information
#[deriving(Show)]
pub struct Info {
    /// The platform identifier
    pub platform_name: PlatformName,
    /// The OpenGL API vesion number
    pub version: Version,
    /// The GLSL vesion number
    pub shading_language: Version,
    /// The extensions supported by the implementation
    pub extensions: HashSet<&'static str>,
}

impl Info {
    fn get() -> Info {
        let info = {
            let platform_name = PlatformName::get();
            let version = Version::parse(get_string(gl::VERSION)).unwrap();
            let shading_language = Version::parse(get_string(gl::SHADING_LANGUAGE_VERSION)).unwrap();
            let extensions = if version >= Version(3, 2, None, "") {
                let num_exts = get_uint(gl::NUM_EXTENSIONS) as gl::types::GLuint;
                range(0, num_exts).map(|i| {
                    unsafe {
                        str::raw::c_str_to_static_slice(
                            gl::GetStringi(gl::EXTENSIONS, i) as *const i8,
                        )
                    }
                }).collect()
            } else {
                // Fallback
                get_string(gl::EXTENSIONS).split(' ').collect()
            };
            Info {
                platform_name: platform_name,
                version: version,
                shading_language: shading_language,
                extensions: extensions,
            }
        };
        info!("Vendor: {}", info.platform_name.vendor);
        info!("Renderer: {}", info.platform_name.renderer);
        info!("Version: {}", info.version);
        info!("Shading Language: {}", info.shading_language);
        info!("Loaded Extensions:")
        for extension in info.extensions.iter() {
            info!("- {}", *extension);
        }
        info
    }

    /// Returns `true` if the implementation supports the extension
    pub fn is_extension_supported(&self, s: &str) -> bool {
        self.extensions.contains_equiv(&s)
    }
}

#[deriving(Eq, PartialEq, Show)]
pub enum ErrorType {
    InvalidEnum,
    InvalidValue,
    InvalidOperation,
    InvalidFramebufferOperation,
    OutOfMemory,
    UnknownError,
}

/// An OpenGL back-end with GLSL shaders
pub struct GlBackEnd {
    caps: device::Capabilities,
    info: Info,
    make_texture: fn(::tex::TextureInfo) -> Texture,
    /// Maps (by the index) from texture name to TextureInfo, so we can look up what texture target
    /// to bind this texture to later. Yuck!
    // Doesn't use a SmallIntMap to avoid the overhead of Option
    samplers: Vec<::tex::SamplerInfo>,
}

impl GlBackEnd {
    /// Load OpenGL symbols and detect driver information
    pub fn new(provider: &device::GlProvider) -> GlBackEnd {
        gl::load_with(|s| provider.get_proc_address(s));
        let info = Info::get();
        let caps = device::Capabilities {
            shader_model: shade::get_model(),
            max_draw_buffers: get_uint(gl::MAX_DRAW_BUFFERS),
            max_texture_size: get_uint(gl::MAX_TEXTURE_SIZE),
            max_vertex_attributes: get_uint(gl::MAX_VERTEX_ATTRIBS),
            uniform_block_supported: info.version >= Version(3, 1, None, "")
                || info.is_extension_supported("GL_ARB_uniform_buffer_object"),
            array_buffer_supported: info.version >= Version(3, 0, None, "")
                || info.is_extension_supported("GL_ARB_vertex_array_object"),
            immutable_storage_supported: info.version >= Version(4, 2, None, "")
                || info.is_extension_supported("GL_ARB_texture_storage"),
            sampler_objects_supported: info.version >= Version(3, 3, None, "")
                || info.is_extension_supported("GL_ARB_sampler_objects"),
        };
        GlBackEnd {
            caps: caps,
            info: info,
            make_texture: if caps.immutable_storage_supported {
                tex::make_with_storage
            } else {
                tex::make_without_storage
            },
            samplers: Vec::new(),
        }
    }

    #[allow(dead_code)]
    fn get_error(&mut self) -> Result<(), ErrorType> {
        match gl::GetError() {
            gl::NO_ERROR => Ok(()),
            gl::INVALID_ENUM => Err(InvalidEnum),
            gl::INVALID_VALUE => Err(InvalidValue),
            gl::INVALID_OPERATION => Err(InvalidOperation),
            gl::INVALID_FRAMEBUFFER_OPERATION => Err(InvalidFramebufferOperation),
            gl::OUT_OF_MEMORY => Err(OutOfMemory),
            _ => Err(UnknownError),
        }
    }

    /// Fails during a debug build if the implementation's error flag was set.
    #[allow(dead_code)]
    fn check(&mut self) {
        debug_assert_eq!(self.get_error(), Ok(()));
    }

    /// Get the OpenGL-specific driver information
    pub fn get_info<'a>(&'a self) -> &'a Info {
        &self.info
    }
}

impl device::ApiBackEnd for GlBackEnd {
    fn get_capabilities<'a>(&'a self) -> &'a device::Capabilities {
        &self.caps
    }

    fn create_buffer(&mut self) -> Buffer {
        let mut name = 0 as Buffer;
        unsafe {
            gl::GenBuffers(1, &mut name);
        }
        info!("\tCreated buffer {}", name);
        name
    }

    fn create_array_buffer(&mut self) -> Result<ArrayBuffer, ()> {
        if self.caps.array_buffer_supported {
            let mut name = 0 as ArrayBuffer;
            unsafe {
                gl::GenVertexArrays(1, &mut name);
            }
            info!("\tCreated array buffer {}", name);
            Ok(name)
        } else {
            error!("\tarray buffer creation unsupported, ignored")
            Err(())
        }
    }

    fn create_shader(&mut self, stage: device::shade::Stage, code: device::shade::ShaderSource) -> Result<Shader, device::shade::CreateShaderError> {
        let (name, info) = shade::create_shader(stage, code, self.get_capabilities().shader_model);
        info.map(|info| {
            let level = if name.is_err() { log::ERROR } else { log::WARN };
            log!(level, "\tShader compile log: {}", info);
        });
        name
    }

    fn create_program(&mut self, shaders: &[Shader]) -> Result<device::shade::ProgramMeta, ()> {
        let (meta, info) = shade::create_program(&self.caps, shaders);
        info.map(|info| {
            let level = if meta.is_err() { log::ERROR } else { log::WARN };
            log!(level, "\tProgram link log: {}", info);
        });
        meta
    }

    fn create_frame_buffer(&mut self) -> FrameBuffer {
        let mut name = 0 as FrameBuffer;
        unsafe {
            gl::GenFramebuffers(1, &mut name);
        }
        info!("\tCreated frame buffer {}", name);
        name
    }

    fn create_texture(&mut self, info: ::tex::TextureInfo) -> Texture {
        (self.make_texture)(info)
    }

    fn create_sampler(&mut self, info: ::tex::SamplerInfo) -> Sampler {
        if self.caps.sampler_objects_supported {
            tex::make_sampler(info)
        } else {
            self.samplers.push(info);
            self.samplers.len() as Sampler - 1
        }
    }

    fn update_buffer(&mut self, buffer: Buffer, data: &device::Blob, usage: device::BufferUsage) {
        gl::BindBuffer(gl::ARRAY_BUFFER, buffer);
        let size = data.get_size() as gl::types::GLsizeiptr;
        let raw = data.get_address() as *const gl::types::GLvoid;
        let usage = match usage {
            device::UsageStatic  => gl::STATIC_DRAW,
            device::UsageDynamic => gl::DYNAMIC_DRAW,
            device::UsageStream  => gl::STREAM_DRAW,
        };
        unsafe {
            gl::BufferData(gl::ARRAY_BUFFER, size, raw, usage);
        }
    }

    fn process(&mut self, request: device::CastRequest) {
        match request {
            device::Clear(data) => {
                let mut flags = match data.color {
                    //gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
                    Some(device::target::Color([r,g,b,a])) => {
                        gl::ClearColor(r, g, b, a);
                        gl::COLOR_BUFFER_BIT
                    },
                    None => 0 as gl::types::GLenum
                };
                data.depth.map(|value| {
                    gl::DepthMask(gl::TRUE);
                    gl::ClearDepth(value as gl::types::GLclampd);
                    flags |= gl::DEPTH_BUFFER_BIT;
                });
                data.stencil.map(|value| {
                    gl::StencilMask(-1);
                    gl::ClearStencil(value as gl::types::GLint);
                    flags |= gl::STENCIL_BUFFER_BIT;
                });
                gl::Clear(flags);
            },
            device::BindProgram(program) => {
                gl::UseProgram(program);
            },
            device::BindArrayBuffer(array_buffer) => {
                if self.caps.array_buffer_supported {
                    gl::BindVertexArray(array_buffer);
                } else {
                    error!("Ignored unsupported GL Request: {}", request)
                }
            },
            device::BindAttribute(slot, buffer, count, el_type, stride, offset) => {
                let gl_type = match el_type {
                    a::Int(_, a::U8, a::Unsigned)  => gl::UNSIGNED_BYTE,
                    a::Int(_, a::U8, a::Signed)    => gl::BYTE,
                    a::Int(_, a::U16, a::Unsigned) => gl::UNSIGNED_SHORT,
                    a::Int(_, a::U16, a::Signed)   => gl::SHORT,
                    a::Int(_, a::U32, a::Unsigned) => gl::UNSIGNED_INT,
                    a::Int(_, a::U32, a::Signed)   => gl::INT,
                    a::Float(_, a::F16) => gl::HALF_FLOAT,
                    a::Float(_, a::F32) => gl::FLOAT,
                    a::Float(_, a::F64) => gl::DOUBLE,
                    _ => {
                        error!("Unsupported element type: {}", el_type);
                        return
                    }
                };
                gl::BindBuffer(gl::ARRAY_BUFFER, buffer);
                let offset = offset as *const gl::types::GLvoid;
                match el_type {
                    a::Int(a::IntRaw, _, _) => unsafe {
                        gl::VertexAttribIPointer(slot as gl::types::GLuint,
                            count as gl::types::GLint, gl_type,
                            stride as gl::types::GLint, offset);
                    },
                    a::Int(a::IntNormalized, _, _) => unsafe {
                        gl::VertexAttribPointer(slot as gl::types::GLuint,
                            count as gl::types::GLint, gl_type, gl::TRUE,
                            stride as gl::types::GLint, offset);
                    },
                    a::Int(a::IntAsFloat, _, _) => unsafe {
                        gl::VertexAttribPointer(slot as gl::types::GLuint,
                            count as gl::types::GLint, gl_type, gl::FALSE,
                            stride as gl::types::GLint, offset);
                    },
                    a::Float(a::FloatDefault, _) => unsafe {
                        gl::VertexAttribPointer(slot as gl::types::GLuint,
                            count as gl::types::GLint, gl_type, gl::FALSE,
                            stride as gl::types::GLint, offset);
                    },
                    a::Float(a::FloatPrecision, _) => unsafe {
                        gl::VertexAttribLPointer(slot as gl::types::GLuint,
                            count as gl::types::GLint, gl_type,
                            stride as gl::types::GLint, offset);
                    },
                    _ => ()
                }
                gl::EnableVertexAttribArray(slot as gl::types::GLuint);
            },
            device::BindIndex(buffer) => {
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, buffer);
            },
            device::BindFrameBuffer(frame_buffer) => {
                gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, frame_buffer);
            },
            device::BindTarget(target, plane) => {
                let attachment = match target {
                    device::target::TargetColor(index) =>
                        gl::COLOR_ATTACHMENT0 + (index as gl::types::GLenum),
                    device::target::TargetDepth => gl::DEPTH_ATTACHMENT,
                    device::target::TargetStencil => gl::STENCIL_ATTACHMENT,
                    device::target::TargetDepthStencil => gl::DEPTH_STENCIL_ATTACHMENT,
                };
                match plane {
                    device::target::PlaneEmpty => gl::FramebufferRenderbuffer
                        (gl::DRAW_FRAMEBUFFER, attachment, gl::RENDERBUFFER, 0),
                    device::target::PlaneSurface(name) => gl::FramebufferRenderbuffer
                        (gl::DRAW_FRAMEBUFFER, attachment, gl::RENDERBUFFER, name),
                    device::target::PlaneTexture(tex, level) => gl::FramebufferTexture
                        (gl::DRAW_FRAMEBUFFER, attachment, tex.name, level as gl::types::GLint),
                    device::target::PlaneTextureLayer(tex, level, layer) => gl::FramebufferTextureLayer
                        (gl::DRAW_FRAMEBUFFER, attachment, tex.name, level as gl::types::GLint, layer as gl::types::GLint),
                }
            },
            device::BindUniformBlock(program, index, loc, buffer) => {
                gl::UniformBlockBinding(program, index as gl::types::GLuint, loc as gl::types::GLuint);
                gl::BindBufferBase(gl::UNIFORM_BUFFER, loc as gl::types::GLuint, buffer);
            },
            device::BindUniform(loc, uniform) => {
                shade::bind_uniform(loc as gl::types::GLint, uniform);
            },
            device::BindTexture(loc, tex, sam) => {
                tex::bind_texture(loc as gl::types::GLuint, tex, sam, self);
            },
            device::SetPrimitiveState(prim) => {
                rast::bind_primitive(prim);
            },
            device::SetDepthStencilState(depth, stencil, cull) => {
                rast::bind_stencil(stencil, cull);
                rast::bind_depth(depth);
            },
            device::SetBlendState(blend) => {
                rast::bind_blend(blend);
            },
            device::UpdateBuffer(buffer, data) => {
                self.update_buffer(buffer, data, device::UsageDynamic);
            },
            device::UpdateTexture(tex, image_info, data) => {
                tex::update_texture(tex, image_info, data);
            },
            device::Draw(start, count) => {
                gl::DrawArrays(gl::TRIANGLES,
                    start as gl::types::GLsizei,
                    count as gl::types::GLsizei);
                self.check();
            },
            device::DrawIndexed(start, count) => {
                let offset = start * (std::mem::size_of::<u16>() as u16);
                unsafe {
                    gl::DrawElements(gl::TRIANGLES,
                        count as gl::types::GLsizei,
                        gl::UNSIGNED_SHORT,
                        offset as *const gl::types::GLvoid);
                }
                self.check();
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use device::Version;

    #[test]
    fn test_version_parse() {
        assert_eq!(Version::parse("1"), Err("1"));
        assert_eq!(Version::parse("1."), Err("1."));
        assert_eq!(Version::parse("1 h3l1o. W0rld"), Err("1 h3l1o. W0rld"));
        assert_eq!(Version::parse("1. h3l1o. W0rld"), Err("1. h3l1o. W0rld"));
        assert_eq!(Version::parse("1.2.3"), Ok(Version(1, 2, Some(3), "")));
        assert_eq!(Version::parse("1.2"), Ok(Version(1, 2, None, "")));
        assert_eq!(Version::parse("1.2 h3l1o. W0rld"), Ok(Version(1, 2, None, "h3l1o. W0rld")));
        assert_eq!(Version::parse("1.2.h3l1o. W0rld"), Ok(Version(1, 2, None, "W0rld")));
        assert_eq!(Version::parse("1.2. h3l1o. W0rld"), Ok(Version(1, 2, None, "h3l1o. W0rld")));
        assert_eq!(Version::parse("1.2.3.h3l1o. W0rld"), Ok(Version(1, 2, Some(3), "W0rld")));
        assert_eq!(Version::parse("1.2.3 h3l1o. W0rld"), Ok(Version(1, 2, Some(3), "h3l1o. W0rld")));
    }
}
