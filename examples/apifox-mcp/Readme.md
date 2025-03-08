cargo build --release --target aarch64-apple-darwin &&

cargo build --release --target x86_64-apple-darwin &&

cargo build --release --target x86_64-unknown-linux-musl && 

cargo build --release --target x86_64-pc-windows-gnu

交叉编译参考：

https://tomshine.hashnode.dev/rust-macos-linux-windows

查看二进制文件大小：
cargo bloat --release --crates

基于 zig 的交叉编译

https://juejin.cn/post/7390645125907267634?searchId=20250308011528DB831BB9693BB6201D9B

cargo zigbuild --target aarch64-unknown-linux-gnu


rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
cargo zigbuild --target universal2-apple-darwin