use std::collections::HashMap;

use tera::Context;

fn default_context() -> Context {
    // add hostname, arch, os, etc
    let mut context = Context::new();

    if let Ok(hostname) = hostname::get() {
        context.insert("hostname", &hostname.to_string_lossy());
    }

    context.insert("arch", std::env::consts::ARCH);

    // OS
    context.insert("os", std::env::consts::OS);

    // Misc info
    context.insert("nproc", &num_cpus::get());

    context
}

pub fn render(template: &str, extra: &HashMap<String, String>) -> Result<String, tera::Error> {
    let mut tera = tera::Tera::default();
    let mut context = default_context();

    if !extra.is_empty() {
        context.extend(Context::from_serialize(extra).unwrap());
    }

    tera.render_str(template, &context)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_render() {
        let template = "Hello, {{ name }}!";
        let mut extra = HashMap::new();
        extra.insert("name".to_string(), "world".to_string());

        let result = render(template, &extra).unwrap();
        assert_eq!(result, "Hello, world!");
    }
}
