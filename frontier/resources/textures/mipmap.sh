svg=$1
width=$2
height=$3
antialias=$4

directory="${svg%%.*}"
rm -rf $directory
mkdir $directory

while true; do

  out=$directory/${width}x${height}.png

  if [ "$antialias" = true ]; then
    magick -size ${width}x${height} -background none $svg -channel alpha -threshold 50% -channel RGB $out
  else
    rsvg-convert -w $width -h $height -f png $svg > $out
  fi

  echo $out

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
