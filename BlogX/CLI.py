from typer import Typer, Option, Argument, Exit
from os import mkdir
from shutil import copytree
from pathlib import Path
from rich import print
from .config import themes, themes_path
from .common import build_site, dev_server

current_path = Path.cwd()

def is_valid_project():
    if not ((current_path / "src").exists() and (current_path / "theme").exists()):
        print("[bold red]Error:[/bold red] Not a valid project directory")
        raise Exit(1)

app = Typer(no_args_is_help=True)

@app.command()
def init(
    project_name: str = Option(..., "--project-name", "-n", help="Name of the project", prompt=True),
    theme: str = Option("BlogX", "--theme", "-t", help="Theme to use", prompt=True),
    blog_name: str = Option("My blog", "--blog-name", "-b", help="Name of the blog", prompt=True),
):
    """Initialize a new project"""
    if theme not in themes:
        # colorful error message
        print(f"[bold red]Error:[/bold red] Theme [bold green]{theme}[/bold green] not found")
        raise Exit(1)

    # Create the project directory
    project_path = Path.cwd() / project_name
    if project_path.exists():
        print(f"[bold red]Error:[/bold red] Project [bold blue]{project_name}[/bold blue] already exists")
        raise Exit(1)
    mkdir(project_path)
    mkdir(project_path / "src")
    mkdir(project_path / "src" / "_global")

    # Copy the theme to the project directory
    theme_path = themes_path / theme
    copytree(theme_path, project_path / "theme")

    # Add index.md and sidebar.md
    with open(project_path / "src" / "index.md", "w", encoding='utf-8') as f:
        f.write("# Welcome to BlogX! This is index.md")
    with open(project_path / "src" / "_global" / "sidebar.md", "w", encoding='utf-8') as f:
        f.writelines([
            "## Shortcuts\n",
            "- [Home](/)\n",
            "- [Journal](/journal)\n",
            "- [About Me](/about)\n",
            "- [Contact](/contact)\n",
            "## Friends\n",
            "- [Friend 1](https://example.com)\n",
            "- [Friend 2](https://example.com)",
        ])
    with open(project_path / "src" / "_global" / "footer.md", "w", encoding='utf-8') as f:
        f.write("Made with BlogX")
    with open(project_path / "src" / "_global" / "header.md", "w", encoding='utf-8') as f:
        f.write(f"# {blog_name}")
    with open(project_path / "src" / "_global" / "BLOGNAME", "w", encoding='utf-8') as f:
        f.write(blog_name)
    print(f"Project [bold red]{project_name}[/bold red] initialized with theme [bold green]{theme}[/bold green]")
    print(f"Run [bold blue]cd {project_name}[/bold blue] to enter the project directory")

@app.command()
def build():
    """Build the site"""
    is_valid_project()
    build_site(current_path / "src", current_path / "dist", current_path / "theme")

@app.command()
def serve():
    """Serve the site"""
    is_valid_project()
    dev_server(current_path / "src", current_path / "dist", current_path / "theme")

@app.command()
def help(
    command: str = Argument(None, help="Command to get help for"),
):
    if command:
        print(f"Help for {command}")
    else:
        print("Help for the CLI")

if __name__ == "__main__":
    app()