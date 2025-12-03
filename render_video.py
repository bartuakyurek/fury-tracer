import subprocess
import os

# Folder containing your PNG frames
#jsonname = "tap"
modelname = "tap"
#output_path = "./outputs/hw3/tap_water/json/"

frames_dir = "./outputs/hw3/tap_water/json/" # os.path.join(output_path, jsonname) 
output_video = f"{modelname}.mp4"

# Frame name like tap_0000.png
frame_pattern = f"{modelname}_%04d.png"  

subprocess.run([
    "ffmpeg",
    "-y",  # overwrite if exists
    "-framerate", "30",  # fps
    "-i", os.path.join(frames_dir, frame_pattern),
    "-c:v", "libx264",
    "-pix_fmt", "yuv420p",
    output_video
])
