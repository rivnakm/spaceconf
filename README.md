# spaceconf
Simple configuration manager for dotfiles and system configuration files

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
[![Test Action](https://github.com/mrivnak/spaceconf/actions/workflows/test.yml/badge.svg)](https://github.com/mrivnak/spaceconf/actions/workflows/test.yml)
![GitHub release (latest SemVer)](https://img.shields.io/github/v/release/mrivnak/spaceconf?display_name=tag&sort=semver)

## Features

### File-Based Configuration Fixtures

```json
{
    "type": "files",
    "files": [
        {
            "src": ".zshrc",
            "dest": "~/.zshrc"
        }
    ]
}
```

### Git-Based Configuration Fixtures

```json
{
    "type": "repository",
    "repository": "https://github.com/mrivnak/neovim-config",
    "reference": {
        "type": "branch",
        "value": "main"
    },
    "path": "~/.config/nvim"
}
```

### File Templating with [Tera](https://keats.github.io/tera/)

```plaintext
{% if hostname == "gentoo-desktop" -%}
monitor=DP-1, 3440x1440@165.00, 2560x0, 1
monitor=DP-2, 2560x1440@239.96, 0x0, 1
{% elif hostname == "gentoo-laptop" -%}
monitor=eDP-1, 2560x1440@165.00, 0x0, 1
{%- endif %}
```

### Secrets support

Secrets that may be stored in existing config files, like an API key for a weather toolbar, can be obfuscated from your dotfiles repo and added on apply

> `<fixture_dir>/secrets.json`

```json
{
    "API_KEY": "12345abcde"
}
```

> `file.conf`

```plaintext
get-data --api-key {{ API_KEY }}
```

## Getting started

### Installing from source

> Requires Rust >=1.67

```bash
git clone https://github.com/mrivnak/spaceconf
cd spaceconf
cargo install --path .
```

### Usage

```bash
spaceconf clone https://github.com/<username>/<dotfiles-repo>
```

You can then create subfolders in `~/.config/spaceconf/` with `fixture.json` files and associated config files. Then run...

```bash
spaceconf apply
```


