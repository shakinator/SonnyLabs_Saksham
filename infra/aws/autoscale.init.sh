#!/usr/bin/env sh

sudo yum install -y docker
sudo systemctl start docker
sudo systemctl enable docker
sudo docker swarm init
