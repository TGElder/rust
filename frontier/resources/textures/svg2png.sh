for file in ls $1/*.svg; do
    file=${file%.svg}
    inkscape -w $2 -h $2 "$file.svg" --export-png "$file.png"
done
