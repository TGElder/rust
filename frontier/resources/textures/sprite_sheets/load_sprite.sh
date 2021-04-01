size=$1

to_png() {

  in=$1
  out=$2
  size=$3

  rsvg-convert -w $size -h $size -f png $in > $out

}

mkdir sprites

for in in ../twemoji/*.svg; do
  out=$(basename $in .svg)
  to_png $in sprites/$out.png $size
done

to_png ../twemoji/derivative/coal.svg sprites/coal.png $size
to_png ../twemoji/derivative/iron.svg sprites/iron.png $size