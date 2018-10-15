#!/bin/sh

protoc --proto_path ./protos --rust_out ./src/protos protos/*.proto
