#!/usr/bin/env bash

yum install -y \
    docker \
    amazon-ecr-credential-helper

mkdir /root/.docker
cat > /root/.docker/config.json <<EOF
{
	"credsStore": "ecr-login"
}
EOF
mkdir -p /home/ec2-user/.docker
cp -r /root/.docker/* /home/ec2-user/.docker/
chown -R ec2-user:ec2-user /home/ec2-user/.docker

sudo systemctl start docker
sudo systemctl enable docker
sudo docker swarm init

aws s3 cp "s3://sonnylabs/infra/web.service.yaml" /home/ec2-user/web.service.yaml

docker pull 215636381729.dkr.ecr.eu-west-1.amazonaws.com/sonnylabs:latest
docker stack deploy \
    --with-registry-auth \
    --compose-file /home/ec2-user/web.service.yaml \
    web

usermod -a -G docker ec2-user
yum install -y postgresql15
