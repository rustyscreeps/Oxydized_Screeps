use oxidized_screeps::{Kernel, Process};

#[test]
fn init_kernel() {
    let k = Kernel::new();  // This creates a new process table

    // usually, it'd be kernel::init(memory_string)

    k.run(); // This kickstarts the kernel from the first process.

    // Somehow, serialize and save to memory
}

#[test]
fn boot_kernel() {
    const MEMORY: &str = "some memory";  // representation TBD

    let k = Kernel::boot(MEMORY);

    k.run();
}
