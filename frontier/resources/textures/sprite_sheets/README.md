# Instructions for generating load sprite sheet

1. Run `./sprite_sheets/load_sprites.sh` to create sprites under `sprites` directory.
1. Use these sprites to create a sprite sheet using TexturePacker
   1. Data Format = JSON (Hash)
   1. Border Padding = 160
   1. Shape Padding = 160
1. Edit JSON and remove the `.png` suffix from hash keys.
1. Run `mipmaps.sh <width> <height>` to create mipmaps in `load` directory. `width` and `height` should be half the dimensions of the sprite sheet image.
1. Delete images created in step 1.
1. Delete `load.png`