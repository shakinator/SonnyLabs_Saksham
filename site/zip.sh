#!/usr/bin/env bash

# include all files not starting with _

files=$(
    find . \
        -type f \
        -and -not -iname '.*' \
        -and -not -iname '_*' \
        -and '(' \
            -iname '*.html' \
            -or -iname '*.mp4' \
            -or -iname '*.jpg' \
            -or -iname '*.jpeg' \
            -or -iname '*.png' \
            -or -iname '*.svg' \
            -or -iname '*.ttf' \
        ')'
)

echo "${files}"
zip site.zip ${files}
    
