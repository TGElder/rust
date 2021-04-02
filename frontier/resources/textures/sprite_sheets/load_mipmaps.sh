width=$1
height=$2

rm -rf load
mkdir load

while true; do

  magick load.png -resize ${width}x${height}\! -channel alpha -threshold 50% load/${width}x${height}.png

  if [ $width -eq 1 ] && [ $height -eq 1 ]; then
    exit
  fi

  width=$(( width / 2 ))
  if [ $width -eq 0 ]; then
    width=1
  fi

  height=$(( height / 2 ))
  if [ $height -eq 0 ]; then
    height=1
  fi

done
