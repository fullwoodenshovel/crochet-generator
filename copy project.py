startdir = ""
ignores = [
    "target/",
    ".gitignore",
    "Cargo.lock",
    "copy project.py",
    "out.txt",
]

import glob
import os

if startdir:
    os.chdir(startdir)
paths = glob.glob(f"**/*.*", recursive=True)
paths = [path for path in paths if not any(path.startswith(ignore) for ignore in ignores)]

result = ""
for path in paths:
    with open(path, "r") as file:
        try:
            text = file.read()
        except UnicodeDecodeError:
            continue
    
    result = result + f"\n\n{path}:\n```\n{text}```"

os.chdir(os.path.dirname(__file__))

with open("out.txt", "w") as file:
    file.write(result)