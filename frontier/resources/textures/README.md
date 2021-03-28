# Instructions for generating load sprite sheet

1. From `/textures`, run `./svg2png.sh twemoji 80` and `./svg2png.sh twemoji/derivative 80`
1. Create a sprite sheet using TexturePacker
   1. Data Format = JSON (Hash)
   1. Border Padding = 4
   1. Shape Padding = 4
1. Edit JSON and remove the `80.png` suffix from hash keys.
1. Delete images created in step 1.

# Instructions for generating body part images (without antialiasing)

1. svg files must have `shape-rendering="crispEdges"` in `<svg>` tag. This may be controlled from Inkscape 0.92 via File -> Document Properties -> Use antialiasing.
1. Inkscape does not respect this tag when exporting; GIMP does. Open the SVG in GIMP at the desired output size ane export to PNG.
