//! Check complete original Copy-product evidence without admission authority.
use rstyle_pdg::{nominal_boundary::resolution, unit_store};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or("Expected raw owner snapshot")?;
    let name = args.next().ok_or("Expected original function name")?;
    let name = name.to_str().ok_or("Expected UTF-8 function name")?;
    if args.next().is_some() {
        return Err("Unexpected probe argument".into());
    }
    let facts = unit_store::read_diagnostic_snapshot(&std::fs::read(path)?)?;
    let owners: Vec<_> = facts.iter().filter(|facts| facts.name == name).collect();
    let [owner] = owners.as_slice() else {
        return Err("Expected exactly one original function".into());
    };
    resolution::probe_copy_product_result(owner)?;
    println!("Complete original Copy-product source proof passed; no admission authority");
    Ok(())
}
