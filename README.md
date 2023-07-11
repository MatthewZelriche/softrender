
<h1 align="center">Softrender</h1>

![](https://github.com/MatthewZelriche/softrender/blob/main/img/header.png?raw=true)

<div align="center">
A simple toy software renderer written in Rust
</div>

## Features

* User-programmable vertex and fragment shaders via Rust traits
* Ability to specify an arbitrary number of vertex attributes to shader programs. All supported Vertex Attribute types will be automatically interpolated without having to write boilerplate code, thanks to Rust's derive macros.
* Perspective-correct vertex attribute interpolation.
* A simple AABB is applied to triangles during rasterization to avoid traversal of unecessary pixels. 

## Getting Started

Construct a new `Renderer`: 

```rust
let mut renderer = Renderer::new(800, 800);
```

The `Renderer` is interacted with primarily through its `draw` function, where you pass the renderer your vertex data, index data, and your shader. To create your shader, first define a struct containing your vertex attributes, for example:

```rust
struct Vertex {
    pos: glam::Vec3,
    color: glam::Vec3,
}
```

This struct specifies the layout of the vertex data you pass to the `Renderer`. You must also define a struct containing the outputs of your vertex shader. These are the values that you want to be interpolated via barycentric coordinates inbetween the vertex and fragment shader stages: 

```rust
#[derive(Clone, Barycentric)]
struct VertexOut {
    color: glam::Vec3,
}
```
Note that you must specify the `Barycentric` derive macro on this struct, so that boilerplate code for interpolating your data can be generated automatically. 

Once you have defined the inputs and outputs for your vertex shader, you can define your programmable 
shader by having it implement the `Shader` trait:

```rust
struct MyShader;
impl Shader<Vertex, VertexOut> for MyShader {
    fn vertex(&self, vertex: &Vertex) -> (glam::Vec4, VertexOut) {
        (
            vertex.pos.extend(1.0),
            VertexOut {
                color: vertex.color,
            },
        )
    }

    fn fragment(&self, inputs: VertexOut) -> glam::UVec3 {
        inputs.color.as_uvec3()
    }
}
```
Note that a vertex position is the only required output of your vertex shader. For the fragment shader, 
the output is a set of RGB color values from 0-255.

All that is left to do is provide the renderer with the required information to execute a single draw call:

```rust
let color_buf = renderer.draw(&mut shader, &vertices, &indices);
```
Providing a set of indices is currently a requirement, even if you do not re-use vertex data. This may change in the future. The `draw` function returns the color buffer, and you can now present the rendered frame however you'd like by accessing the raw array of pixel values. The provided examples use the `softbuffer` and `winit` crates to render to a window without requiring GPU acceleration.

For more information on using this crate, see the `examples` subdirectory for several complete examples, including more complicated use cases such as loading and rendering 3D model data. 


## Examples

### 1. Hello Triangle 

![](https://github.com/MatthewZelriche/softrender/blob/main/img/hello_triangle.png?raw=true)

### 2. Texture Rendering

![](https://github.com/MatthewZelriche/softrender/blob/main/img/texture.png?raw=true)

### 3. Model Loading

![](https://github.com/MatthewZelriche/softrender/blob/main/img/model_load.png?raw=true)


## Licensing Information

This project is licensed under the MIT License. See the LICENSE file for details. 

Other credits are as follows:

1. `res/coyote.jpg`, `res/coyote.obj`: [alitural, CC BY 4.0](https://sketchfab.com/3d-models/coyote-d470f716e00f484b853033ed2d4fdfca)
2. `res/texture.png`: [Generated by Test Grid Generator, Wahooney, CC0](https://wahooney.itch.io/texture-grid-generator)
3. `res/teapot.obj`: [Martin Newell, Kenzie Lamar, CC0](https://casual-effects.com/data/)


