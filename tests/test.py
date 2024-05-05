import subprocess

from typing import List

def assert_eq(a, b):
    if a != b:
        print(f"{a} != {b}")
        raise Exception("assertion failed")

def spaceconf(args: List[str]) -> str:
    result = subprocess.run(["target/release/spaceconf"] + args, capture_output=True, check=False)
    if result.returncode != 0:
        print(result.stderr.decode("utf-8"))
        raise Exception("spaceconf failed")

    return result.stdout.decode("utf-8")

def main():
    output = spaceconf(["check"])
    output = output.split("\n")
    assert_eq(output[0], '"/root/.zshrc" does not exist')
    assert_eq(output[1], '"/root/.zprofile" does not exist')

    output = spaceconf(["apply"])
    output = spaceconf(["check"])
    output = output.split("\n")
    assert_eq(output[0], '"/root/.zshrc" is up to date')
    assert_eq(output[1], '"/root/.zprofile" is up to date')

    subprocess.run("echo hello >> /root/.config/spaceconf/zsh/zshrc", shell=True, check=True)
    subprocess.run("echo hello >> /root/.config/spaceconf/zsh/zprofile", shell=True, check=True)
    output = spaceconf(["check"])
    output = output.split("\n")
    assert_eq(output[0], '"/root/.zshrc" is NOT up to date')
    assert_eq(output[1], '"/root/.zprofile" is NOT up to date')

    print("All tests passed!")

if __name__ == "__main__":
    main()

