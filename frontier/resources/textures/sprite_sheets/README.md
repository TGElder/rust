# Instructions for generating resources sprite sheet

1. From `/textures`, run `./svg2png.sh twemoji 80` and `./svg2png.sh twemoji/derivative 80`
1. Create a sprite sheet using TexturePacker
   1. Data Format = JSON (Hash)
   1. Border Padding = 4
   1. Shape Padding = 4
1. Edit `resources.json` and remove the `80.png` suffix from hash keys.
1. Delete images created in step 1.