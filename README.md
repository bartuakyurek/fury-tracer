
<div style="text-align: center;">
  <h1>˚ˋঌ˖ Fury Tracer ˋঌ </h1>
</div>


This is a ray tracer developed for METU graduate level course CENG 795. If you'd like to read learning logs of this work, see [my Notion page](https://bartuakyurek.notion.site/Journey-of-a-Rust-Ray-Tracer-29bf07c7cdad8045ae34f2f68f4d7301).


Render default .json:
---
For fastest run:
``$ cargo run --release``

For debugging (slow):
``$ RUST_LOG=debug cargo run``

For unit tests:
``$ cargo test``

For more suggestions to improve code:
``$ cargo clippy``

---

Render a specific .json:
``$ cargo run --release ./path/to/your.json``

Render all .json files under a directory:
``$ cargo run --release ./path/to/folder``

---

> [!IMPORTANT]
> Binaries are placed under ./target/debug/ or ./target/release depending on cargo run commands above. By default it is under debug but for faster runs compiling with --release is recommended.

![Elmo Fire](./assets/elmofire.png)

# TODO 
---
- [ ] Currently epsilon is added before transforming the ray, which doesn't cause discrepency in expected output but should we consider
adding epsilon after the transformed rays?
- [ ] Cache inverse transforms as calling .inverse( ) is inefficient 
- [x] Acne problem in mirror_room.json (Fixed by setting intensity 0 point light id = 2)
- [x] Glass less reflective in instanced meshes (metal_glass_plates.json) (fixed by not normalizing ray direction after transform)
- [x] Glass material looks less bright than expected, why?
- [x] Vertex indices start from 1, not 0, make sure to handle that correctly
- [x] Ambient light is declared only once, change implementation to return a single vec3, Not vector of vec3. 
- [ ] Consider explicitly marking your function with #[inline] or #[inline(always)] to see if it improves performance. (source: https://softwaremill.com/rust-static-vs-dynamic-dispatch/)
- [ ] Fix Fresnel computations done twice, see comments or commits for thoughts on it.
- [ ] I think boilerplate required for implementing material can be reduced if material trait had default ambient( ) diffuse( ) specular( )
but what is missing is the struct to hold these coefficients, so maybe we could add &MaterialCommon, a struct to hold these info and then
pass it to the trait function so it knows what data to access. But the problem is we do not know which material struct to call at renderer
it is dyn Material so ... I guess this wouldn't work. 
- [ ] Every shape in this class has material_idx so there could be a struct
    dedicated to such data unrelated to shapes but required for HitRecord. But
    Rust does not allow implicit inheritance so we cannot just:
    ```
    struct Shape { // better naming here? 
        material: Index,    
    }

    struct SomeShape : Shape {
        some_other_member: SomeType,
    }
```
    What is recommended is to use composition, e.g.

    ```struct ShapeData {
        material: Index,
    }

    struct SomeShape {
        _data: ShapeData,
        some_other_member: SomeType,
    }```

    however I'm not sure if this is helpful in our case or if it adds unnecessary complexity.
    In case this becomes necessary, check out this crate https://docs.rs/delegate/latest/delegate/. 
    Update: I completely forgot why I had this discussion, now I remember another reason: some materials share reflectance coefficients and I'd like to have specular( ) diffuse( ) functions taking material.specular_rf but traits do not allow holding data, rather we could use composition to have that information (I guess a struct like BRDF?)

    ---
    ## How to add more material? (TODO: should reduce the boilerplate here)
    - Declare your CustomMaterialStruct
    - impl material::Material for CustomMaterialStruct 
    - Add match arm to scene::parse_single_material( ) using _type value of JSON (TODO: automatize that?)
    - Add match arm to scene::get_color( ) for custom reflect / refract 

    ----
    ### How to render videos?

    Example usage
    ```
    $ RUST_LOG=off cargo run --release ./inputs/hw2/raven/camera_zoom_david/
    ```
    this will output under ./outputs/hw2/raven/camera_zoom_david/ 
    use this path to edit render_video.py and run ``python render_video.py`` (TODO: add args to ease usage)