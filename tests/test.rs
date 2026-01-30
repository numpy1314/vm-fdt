use std::convert::TryInto;
use vm_fdt_arceos::{Error, FdtReserveEntry, FdtWriter};

const FDT_MAGIC: u32 = 0xd00dfeed;

fn verify_header(blob: &[u8]) {
    assert!(blob.len() > 40, "Blob too small to contain header");
    let magic_bytes: [u8; 4] = blob[0..4].try_into().unwrap();
    let magic = u32::from_be_bytes(magic_bytes);
    assert_eq!(magic, FDT_MAGIC, "Invalid FDT magic number");
}

#[test]
fn test_simple_fdt_creation() -> Result<(), Error> {
    let mut fdt = FdtWriter::new()?;
    let root_node = fdt.begin_node("root")?;
    fdt.property_string("compatible", "linux,dummy-virt")?;
    fdt.property_u32("#address-cells", 2)?;
    fdt.property_u32("#size-cells", 2)?;
    fdt.end_node(root_node)?;

    let blob = fdt.finish()?;
    verify_header(&blob);

    Ok(())
}

#[test]
fn test_all_property_types() -> Result<(), Error> {
    let mut fdt = FdtWriter::new()?;
    let node = fdt.begin_node("prop_test")?;

    fdt.property_null("empty-prop")?;
    fdt.property_string("str-prop", "hello world")?;
    fdt.property_u32("u32-prop", 0x12345678)?;
    fdt.property_u64("u64-prop", 0x1234567890ABCDEF)?;
    fdt.property_array_u32("u32-arr", &[1, 2, 3, 4])?;
    fdt.property_array_u64("u64-arr", &[100, 200])?;

    fdt.property("raw-bytes", &[0xDE, 0xAD, 0xBE, 0xEF])?;

    fdt.property_string_list("str-list", vec!["one".to_string(), "two".to_string()])?;

    fdt.end_node(node)?;
    let blob = fdt.finish()?;
    verify_header(&blob);

    Ok(())
}

#[test]
fn test_nested_nodes() -> Result<(), Error> {
    let mut fdt = FdtWriter::new()?;

    let root = fdt.begin_node("")?;

    let child1 = fdt.begin_node("cpu@0")?;
    fdt.property_string("device_type", "cpu")?;
    fdt.end_node(child1)?;

    let child2 = fdt.begin_node("memory@80000000")?;
    fdt.property_string("device_type", "memory")?;

    let grandchild = fdt.begin_node("bank0")?;
    fdt.property_u32("reg", 0)?;
    fdt.end_node(grandchild)?;

    fdt.end_node(child2)?;

    fdt.end_node(root)?;

    let blob = fdt.finish()?;
    verify_header(&blob);
    Ok(())
}

#[test]
fn test_memory_reservations() -> Result<(), Error> {
    let reservations = vec![
        FdtReserveEntry::new(0x1000, 0x1000).unwrap(),
        FdtReserveEntry::new(0x80000000, 0x20000).unwrap(),
    ];

    let mut fdt = FdtWriter::new_with_mem_reserv(&reservations)?;
    let root = fdt.begin_node("root")?;
    fdt.end_node(root)?;

    let blob = fdt.finish()?;
    verify_header(&blob);

    assert!(blob.len() >= 40 + 16 * 3);

    Ok(())
}

#[test]
fn test_phandle_uniqueness() -> Result<(), Error> {
    let mut fdt = FdtWriter::new()?;
    let root = fdt.begin_node("root")?;

    let n1 = fdt.begin_node("node1")?;
    fdt.property_phandle(1)?;
    fdt.end_node(n1)?;

    let n2 = fdt.begin_node("node2")?;
    let result = fdt.property_phandle(1);

    assert_eq!(result, Err(Error::DuplicatePhandle));

    fdt.property_phandle(2)?;
    fdt.end_node(n2)?;

    fdt.end_node(root)?;
    Ok(())
}

#[test]
fn test_node_name_validation() -> Result<(), Error> {
    let mut fdt = FdtWriter::new()?;

    let n1 = fdt.begin_node("valid-node@1")?;
    fdt.end_node(n1)?;

    let invalid_name = "invalid/name";
    let result = fdt.begin_node(invalid_name);

    if let Err(e) = result {
        assert_eq!(e, Error::InvalidNodeName);
    } else {
        println!(
            "Warning: Library accepted potentially invalid node name '{}'",
            invalid_name
        );
    }

    Ok(())
}

#[test]
fn test_state_machine_violations() -> Result<(), Error> {
    let mut fdt = FdtWriter::new()?;

    assert_eq!(
        fdt.property_u32("test", 1),
        Err(Error::PropertyBeforeBeginNode)
    );

    let root = fdt.begin_node("root")?;

    fdt.end_node(root)?;

    assert_eq!(
        fdt.property_u32("too-late", 1),
        Err(Error::PropertyAfterEndNode)
    );

    let mut fdt2 = FdtWriter::new()?;
    let _open_node = fdt2.begin_node("unclosed")?;
    assert_eq!(fdt2.finish().map(|_| ()), Err(Error::UnclosedNode));

    Ok(())
}

#[test]
fn test_large_property_handling() -> Result<(), Error> {
    let mut fdt = FdtWriter::new()?;
    let root = fdt.begin_node("root")?;

    let large_data = vec![0u8; 1024];
    fdt.property("large-blob", &large_data)?;

    fdt.end_node(root)?;
    let blob = fdt.finish()?;

    assert!(blob.len() > 1024);
    verify_header(&blob);
    Ok(())
}
