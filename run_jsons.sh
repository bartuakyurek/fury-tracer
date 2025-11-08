#!/usr/bin/env bash

root_path="./inputs/hw1"

# List of JSON files to run
json_files=(
    "spheres_mirror.json"
    "simple.json"
    "spheres.json"
    "two_spheres.json"
    "cornellbox.json"
    "cornellbox_recursive.json"
    #"scienceTree.json"
    #"scienceTree_glass.json"
    #"akif_uslu/ton_Roosendaal_smooth.json"
    #"raven/rt_david.json"
    #"raven/rt_raven.json"
    #"raven/rt_utahteapot_mug_ceng.json"
)

for json_file in "${json_files[@]}"; do
    full_path="${root_path}/${json_file}"
    if [ -f "$full_path" ]; then
        echo "Running cargo for $full_path ..."
        RUSTFLAGS="-Awarnings" cargo run --release -- "$full_path"
        echo "---------------------------------------"
    else
        echo "File not found: $full_path"
    fi
done