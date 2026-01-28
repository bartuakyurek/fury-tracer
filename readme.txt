

All blogs available at https://www.notion.so/bartuakyurek/Journey-of-a-Rust-Ray-Tracer-29bf07c7cdad8045ae34f2f68f4d7301
Source available at https://github.com/bartuakyurek/fury-tracer

-----------------------------------------------------------------------------
FOR PROJECT
---------------
There exists a json file implementation as well (./inputs/red-circle.json) but 
it is incomplete. For now using .png as input works the quickest, e.g.:

$ QUICK_PNG=1 cargo run --release ./inputs/images/cornell-box-2d.png ./output.png

(don't forget the QUICK_PNG flag in the beginning)

FOR HOMEWORKS
----------------
To render a specific .json: 
$ cargo run --release ./path/to/your.json

To batch render all .json files under a directory: 
$ cargo run --release ./path/to/folder

-----------------------------------------------------------------------------
Example usage for rendering frames of a video:
$ RUST_LOG=off cargo run --release ./inputs/raven/camera_zoom_david/

this will output under ./outputs/raven/camera_zoom_david/ 
use this path to edit render_video.py and run ``python render_video.py``

-----------------------------------------------------------------------------
Note that cargo with --release flag will place executable under target/release/
Instead of cargo executable can also be used directly
./raytracer ./path/to/json/or/a/folder

-----------------------------------------------------------------------------

