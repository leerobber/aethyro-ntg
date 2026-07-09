fn main() {
    let cap = ntg_kernel::ternary_capability();
    println!("NTG Kernel Host Booting…");
    println!(
        "capability v{}  scalar={} packed={} bit_sliced={} sparse={} runtime={}",
        cap.version,
        cap.scalar_supported,
        cap.packed_supported,
        cap.bit_sliced_supported,
        cap.sparse_bit_sliced_supported,
        cap.native_parallel_forward_supported
    );
    println!("density micro-bench: cargo run --release --bin density_bench");
}
