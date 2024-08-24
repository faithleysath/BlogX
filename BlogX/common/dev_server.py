from pathlib import Path
from livereload import Server
import importlib.util
import logging
from typer import Exit

current_path = Path.cwd()
theme_path = current_path / "themes" / "BlogX"
builder_path = theme_path / "builder.py"
output_dir = current_path / "dist"

def is_valid_theme(theme_path):
    if not (theme_path / "builder.py").exists():
        print("[bold red]Error:[/bold red] Not a valid theme directory")
        raise Exit(1)

def get_builder(): # load the builder.py file from the theme
    is_valid_theme(theme_path)
    spec = importlib.util.spec_from_file_location("builder", builder_path)
    builder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(builder)
    return builder

def on_modified(filename: str = "Null"):
    logging.info(f"{filename} modified")
    get_builder().build(current_path / "src", output_dir, Path(filename))

def dev_server():
    get_builder().build(current_path / "src", output_dir)
    # create a server instance
    server = Server()
    # watch the src directory and rebuild the site when a file is modified
    server.watch(current_path / "src", on_modified)
    # start the server
    server.serve(root=output_dir)