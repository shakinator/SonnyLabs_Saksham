#!/usr/bin/env bash

aws ecr get-login-password \
    | docker login \
        --username AWS --password-stdin \
        215636381729.dkr.ecr.eu-west-1.amazonaws.com
