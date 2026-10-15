pub fn build() -> Vec<u8> {
    include_bytes!(concat!(env!("OUT_DIR"), "/stub.bin")).to_vec()
}
