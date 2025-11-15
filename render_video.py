import subprocess
import os

# Folder containing your PNG frames
jsonname = "camera_zoom_david"
modelname = "davids_camera_zoom"

output_path = "./outputs/hw2/raven/"

frames_dir = os.path.join(output_path, jsonname) 
output_video = f"{modelname}.mp4"

# Frame name like windmill_000.png
frame_pattern = f"{modelname}_%03d.png"  

subprocess.run([
    "ffmpeg",
    "-y",  # overwrite if exists
    "-framerate", "30",  # fps
    "-i", os.path.join(frames_dir, frame_pattern),
    "-c:v", "libx264",
    "-pix_fmt", "yuv420p",
    output_video
])
