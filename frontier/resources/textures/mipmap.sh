svg=$1
width=$2
height=$3
directory="${svg%%.*}"
rm -rf $directory
mkdir $directory
while true; do

  out=$directory/${width}x${height}.png

#   magick -background none $1 -resize ${width}x${height} -channel RGB $out
  rsvg-convert -w $width -h $height -f png $1 > $out

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
