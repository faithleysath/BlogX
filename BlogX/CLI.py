from typer import Typer, Option, Argument


app = Typer(no_args_is_help=True)

@app.command()
def init(
    project_name: str = Option(..., "--project-name", "-n", help="Name of the project", prompt=True),
    template: str = Option("blogx", "--template", "-t", help="Template to use"),
):
    print(f"Creating project {project_name} with template {template}")

@app.command()
def help(
    command: str = Argument(None, help="Command to get help for"),
):
    if command:
        print(f"Help for {command}")
    else:
        print("Help for the CLI")