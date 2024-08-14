#!/usr/bin/env bash

for i in $(seq "${NPROCS}")
do
    echo $i
    /app/server &
done

wait
