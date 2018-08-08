## Building

***These instructions will help build trow on Ubuntu.***
- Install Rust Nightly
```
curl -s https://static.rust-lang.org/rustup.sh | sh -s -- --channel=nightly
```
- Firstly clone the trow repo.
```
git clone git://github.com/ContainerSolutions/trow
```
 - Install some prerequisites
```
sudo apt install gcc cmake golang unzip libssl-dev docker-io
```
- Build the key and certificate
```
cd install/self-cert/
./make-certs.sh
mkdir ../../certs
cp domain.key ca.crt ../../certs/
cd ../..
```
- Download Protocol Buffers and drop the exe in local bin
```
mkdir protoc
cd protoc
sudo curl -o protoc.zip -sSL https://github.com/google/protobuf/releases/download/v3.5.1/protoc-3.5.1-linux-x86_64.zip && unzip protoc.zip
sudo cp bin/protoc /usr/local/bin/
sudo chmod ugo+x /usr/local/bin/protoc
```
- Finally build and run tests
```
cd ..
cargo test
```
