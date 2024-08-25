from pathlib import Path
import importlib.util
import shutil
from typer import Exit

current_path = Path.cwd()
theme_path = current_path / "themes" / "BlogX"
builder_path = theme_path / "builder.py"
output_dir = current_path / "dist"

def is_valid_theme(theme_path):
    if not (theme_path / "builder.py").exists():
        print("[bold red]Error:[/bold red] Not a valid theme directory")
        raise Exit(1)

def build():
    is_valid_theme(theme_path)
    spec = importlib.util.spec_from_file_location("builder", builder_path)
    builder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(builder)
    # remove the output directory if it exists
    if output_dir.exists():
        shutil.rmtree(output_dir)
    builder.build(current_path / "src", output_dir)