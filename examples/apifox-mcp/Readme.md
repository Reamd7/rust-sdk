cargo build --release --target aarch64-apple-darwin

cargo build --release --target x86_64-apple-darwin

cargo build --release --target x86_64-unknown-linux-musl &

cargo build --release --target x86_64-pc-windows-gnu

交叉编译参考：

https://tomshine.hashnode.dev/rust-macos-linux-windows

查看二进制文件大小：
cargo bloat --release --crates