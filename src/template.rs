use std::collections::HashMap;

use tera::Context;

fn default_context() -> Context {
    // add hostname, arch, os, etc
    let mut context = Context::new();

    if let Ok(hostname) = hostname::get() {
        context.insert("hostname", &hostname.to_string_lossy());
    }

    // Arch
    #[cfg(target_arch = "x86_64")]
    context.insert("arch", "x86_64");

    #[cfg(target_arch = "aarch64")]
    context.insert("arch", "aarch64");

    // OS
    #[cfg(target_os = "linux")]
    context.insert("os", "linux");

    #[cfg(target_os = "macos")]
    context.insert("os", "macos");

    #[cfg(target_os = "windows")]
    context.insert("os", "windows");

    // Misc info
    context.insert("nproc", &num_cpus::get());
    context.insert("wsl", &wsl::is_wsl());

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
