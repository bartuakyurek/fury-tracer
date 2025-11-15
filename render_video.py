import subprocess
import os

# Folder containing your PNG frames
modelname = "windmill"
frames_dir = "./outputs/hw2/akif_uslu/" + modelname  + "/input/"
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
