use tokio_util::codec::Decoder;

#[derive(Default)]
pub struct JsonRpcFrameCodec;

// JsonRpcFrameCodec 解码器实现
// JsonRpcFrameCodec decoder implementation
impl Decoder for JsonRpcFrameCodec {
    type Item = tokio_util::bytes::Bytes;
    type Error = tokio::io::Error;

    // 解码方法
    // Decode method
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        // 查找换行符
        // Find newline character
        if let Some(end) = src
            .iter()
            .enumerate()
            .find_map(|(idx, &b)| (b == b'\n').then_some(idx))
        {
            // 分割行
            // Split line
            let line = src.split_to(end);
            let _char_next_line = src.split_to(1);
            Ok(Some(line.freeze()))
        } else {
            Ok(None)
        }
    }
}
