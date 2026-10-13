pub fn build() -> Result<Vec<u8>, String> {
    const REGION_LEN: usize = 256;
    let stub: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/stub.bin"));
    if stub.len() > REGION_LEN {
        return Err(format!(
            "stub is {} bytes, over the {REGION_LEN}-byte stub region",
            stub.len()
        ));
    }
    let mut out = stub.to_vec();
    out.resize(REGION_LEN, 0);
    Ok(out)
}
