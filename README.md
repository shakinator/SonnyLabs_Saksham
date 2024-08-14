SonnyLabs
=========

| Path | Description |
| ---- | ----------- |
| service/ | SonnyLabs detection service |
| api/     | APIs for calling SonnyLabs detection service |
| ml/      | ML workbooks, data, training |
| site/    | Static website |


Quickstart
==========

Clone the repo.

Install Docker and Python in your system.

Go into the `service/` directory (found in the repo).
```sh
$ cd service/
```

Build the Docker container
```sh
$ docker build . -t sonnylabs:latest
```

Run the Docker container with port 3000 exposed
```sh
$ docker run -it -p 3000:3000 sonnylabs:latest
```

Next, go to the `api/python` directory
```sh
$ cd ../api/python
```
Install the pre-requisites
```sh
$ pip install -r requirements.txt
```
And run the example
```sh
$ python3 main.py
```
