#!/usr/bin/env bash

s3_infra="s3://sonnylabs/infra"

../../tools/merge-yaml web.service.yaml _web.service.prod.yaml \
    | aws s3 cp - "${s3_infra}"/web.service.yaml

aws s3 cp web.init.sh "${s3_infra}"/web.init.sh
