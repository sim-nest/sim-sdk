fn main() {
    let data = [1u8, 2, 3];
    let borrowed = sim::kernel::NativeAbiBorrowedBytes::borrow(&data);

    fn destroy(_: sim::kernel::NativeAbiOwnedBytes) {}

    destroy(borrowed);
}
