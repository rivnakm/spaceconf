use log::error;
use similar::TextDiff;

use crate::{
    fixture::{Fixture, FixtureType},
    template,
};

pub fn diff_fixtures(fixtures: Vec<Fixture>) {
    for fixture in fixtures {
        if fixture.skip() {
            continue;
        }
        let FixtureType::Files(setup) = fixture.fixture_type else {
            continue;
        };

        for file in setup.files {
            let Some(src) = file.src.clone().resolve() else {
                continue;
            };
            let Some(dest) = file.dest.clone().resolve() else {
                continue;
            };

            let input = std::fs::read_to_string(&src)
                .inspect_err(|_| error!("failed to read source file: {}", &src.to_string_lossy()))
                .unwrap();
            let input = template::render(&input, &setup.secrets).unwrap();

            let output = if dest.exists() {
                std::fs::read_to_string(&dest)
                    .inspect_err(|_| {
                        error!(
                            "failed to read destination file: {}",
                            &dest.to_string_lossy()
                        )
                    })
                    .unwrap()
            } else {
                String::new()
            };

            let diff = TextDiff::from_lines(&input, &output);
            if diff.ratio() == 1.0 {
                continue;
            }

            let mut unified = diff.unified_diff();
            unified.header(&src.to_string_lossy(), &dest.to_string_lossy());

            let stdout = std::io::stdout();
            unified.to_writer(stdout.lock()).unwrap();
        }
    }
}
