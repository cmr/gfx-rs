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

use r = device::rast;
use device::target::Color;
use gl;

pub fn bind_primitive(p: r::Primitive) {
    gl::FrontFace(match p.front_face {
        r::Clockwise => gl::CW,
        r::CounterClockwise => gl::CCW,
    });

    let (gl_draw, gl_offset) = match p.method {
        r::Point => (gl::POINT, gl::POLYGON_OFFSET_POINT),
        r::Line(width) => {
            gl::LineWidth(width);
            (gl::LINE, gl::POLYGON_OFFSET_LINE)
        },
        r::Fill(cull) => {
            match cull {
                r::CullNothing => gl::Disable(gl::CULL_FACE),
                r::CullFront => {
                    gl::Enable(gl::CULL_FACE);
                    gl::CullFace(gl::FRONT);
                },
                r::CullBack => {
                    gl::Enable(gl::CULL_FACE);
                    gl::CullFace(gl::BACK);
                },
            }
            (gl::FILL, gl::POLYGON_OFFSET_FILL)
        },
    };

    gl::PolygonMode(gl::FRONT_AND_BACK, gl_draw);

    match p.offset {
        r::Offset(factor, units) => {
            gl::Enable(gl_offset);
            gl::PolygonOffset(factor, units as gl::types::GLfloat);
        },
        r::NoOffset => gl::Disable(gl_offset),
    }
}

fn map_comparison(cmp: r::Comparison) -> gl::types::GLenum {
    match cmp {
        r::Never        => gl::NEVER,
        r::Less         => gl::LESS,
        r::LessEqual    => gl::LEQUAL,
        r::Equal        => gl::EQUAL,
        r::GreaterEqual => gl::GEQUAL,
        r::Greater      => gl::GREATER,
        r::NotEqual     => gl::NOTEQUAL,
        r::Always       => gl::ALWAYS,
    }
}

pub fn bind_depth(depth: Option<r::Depth>) {
    match depth {
        Some(d) => {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(map_comparison(d.fun));
            gl::DepthMask(if d.write {gl::TRUE} else {gl::FALSE});
        },
        None => gl::Disable(gl::DEPTH_TEST),
    }
}

fn map_operation(op: r::StencilOp) -> gl::types::GLenum {
    match op {
        r::OpKeep          => gl::KEEP,
        r::OpZero          => gl::ZERO,
        r::OpReplace       => gl::REPLACE,
        r::OpIncrementClamp=> gl::INCR,
        r::OpIncrementWrap => gl::INCR_WRAP,
        r::OpDecrementClamp=> gl::DECR,
        r::OpDecrementWrap => gl::DECR_WRAP,
        r::OpInvert        => gl::INVERT,
    }
}

pub fn bind_stencil(stencil: Option<r::Stencil>, cull: r::CullMode) {
    fn bind_side(face: gl::types::GLenum, side: r::StencilSide) {
        gl::StencilFuncSeparate(face, map_comparison(side.fun),
            side.value as gl::types::GLint, side.mask_read as gl::types::GLuint);
        gl::StencilOpSeparate(face, map_operation(side.op_fail),
            map_operation(side.op_depth_fail), map_operation(side.op_pass));
    }
    match stencil {
        Some(s) => {
            gl::Enable(gl::STENCIL_TEST);
            if cull != r::CullFront {
                bind_side(gl::FRONT, s.front);
            }
            if cull != r::CullBack {
                bind_side(gl::BACK, s.back);
            }
        }
        None => gl::Disable(gl::STENCIL_TEST),
    }
}


fn map_equation(eq: r::Equation) -> gl::types::GLenum {
    match eq {
        r::FuncAdd    => gl::FUNC_ADD,
        r::FuncSub    => gl::FUNC_SUBTRACT,
        r::FuncRevSub => gl::FUNC_REVERSE_SUBTRACT,
        r::FuncMin    => gl::MIN,
        r::FuncMax    => gl::MAX,
    }
}

fn map_factor(factor: r::Factor) -> gl::types::GLenum {
    match factor {
        r::Factor(r::Normal,  r::Zero)        => gl::ZERO,
        r::Factor(r::Inverse, r::Zero)        => gl::ONE,
        r::Factor(r::Normal,  r::SourceColor) => gl::SRC_COLOR,
        r::Factor(r::Inverse, r::SourceColor) => gl::ONE_MINUS_SRC_COLOR,
        r::Factor(r::Normal,  r::SourceAlpha) => gl::SRC_ALPHA,
        r::Factor(r::Inverse, r::SourceAlpha) => gl::ONE_MINUS_SRC_ALPHA,
        r::Factor(r::Normal,  r::DestColor)   => gl::DST_COLOR,
        r::Factor(r::Inverse, r::DestColor)   => gl::ONE_MINUS_DST_COLOR,
        r::Factor(r::Normal,  r::DestAlpha)   => gl::DST_ALPHA,
        r::Factor(r::Inverse, r::DestAlpha)   => gl::ONE_MINUS_DST_ALPHA,
        r::Factor(r::Normal,  r::ConstColor)  => gl::CONSTANT_COLOR,
        r::Factor(r::Inverse, r::ConstColor)  => gl::ONE_MINUS_CONSTANT_COLOR,
        r::Factor(r::Normal,  r::ConstAlpha)  => gl::CONSTANT_ALPHA,
        r::Factor(r::Inverse, r::ConstAlpha)  => gl::ONE_MINUS_CONSTANT_ALPHA,
        r::Factor(r::Normal,  r::SourceAlphaSaturated) => gl::SRC_ALPHA_SATURATE,
        _ => fail!("Unsupported blend factor: {}", factor),
    }
}

pub fn bind_blend(blend: Option<r::Blend>) {
    match blend {
        Some(b) => {
            gl::Enable(gl::BLEND);
            gl::BlendEquationSeparate(
                map_equation(b.color.equation),
                map_equation(b.alpha.equation));
            gl::BlendFuncSeparate(
                map_factor(b.color.source),
                map_factor(b.color.destination),
                map_factor(b.alpha.source),
                map_factor(b.alpha.destination));
            let Color([r, g, b, a]) = b.value;
            gl::BlendColor(r, g, b, a);
        },
        None => gl::Disable(gl::BLEND),
    }
}
