use fixture::{Fixture, FixtureType};

mod fixture;
pub mod git;
pub mod ops;
mod repo;
mod template;

pub fn list_fixtures(fixtures: Vec<Fixture>) {
    for fixture in fixtures {
        match fixture.fixture_type {
            FixtureType::Files(setup) => {
                println!("Fixture: {}", fixture.name);
                if setup.root {
                    println!("  Root: true");
                }
                for file in setup.files {
                    if let Some(dest) = file.dest.resolve() {
                        println!("  File: {}", dest.display());
                    }
                }
            }
            FixtureType::Repository(setup) => {
                println!("Fixture: {}", setup.repository);
                println!("  Reference: {:?}", setup.reference);
                println!("  Path: {}", setup.path.display());
            }
        }
    }
}
