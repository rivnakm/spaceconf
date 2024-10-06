import semver
import sys
import toml

tag_ver = sys.argv[1]

with open("Cargo.toml", "r") as f:
    cargo_toml = toml.load(f)

ver = cargo_toml["package"]["version"]

if semver.compare(ver, tag_ver) == 1:
    sys.exit(0)
else:
    sys.exit(1)
