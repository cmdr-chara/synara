//! Inspect metadata only. This example never installs or executes an agent.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: inspect_index REGISTRY.json")?;
    let bytes = std::fs::read(path)?;
    let registry = synara_registry::Registry::parse(&bytes)?;
    let platform = synara_registry::Platform::current()?;
    let mut available = 0;
    for agent in &registry.agents {
        match agent.plan(platform) {
            Ok(plan) => {
                available += 1;
                println!("{} {}: {}", agent.id, agent.version, plan.origin());
            }
            Err(error) => println!("{} {}: {error}", agent.id, agent.version),
        }
    }
    println!(
        "{} valid entries, {} available for {}",
        registry.agents.len(),
        available,
        platform.key()
    );
    Ok(())
}
