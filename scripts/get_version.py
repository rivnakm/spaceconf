import toml

with open("Cargo.toml", "r") as f:
    cargo_toml = toml.load(f)
    print(cargo_toml["package"]["version"])

