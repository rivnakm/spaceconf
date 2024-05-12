$version = $args[0].replace("v", "")

$files = @(
    "./Cargo.toml"
)

foreach ($file in $files) {
    ((Get-Content -path $file -Raw) -replace '0.0.0', $version) | Set-Content -Path $file
}
