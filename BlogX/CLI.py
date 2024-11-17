VERSION = "0.2.9"
from typer import Typer, Option, Argument, Exit
from os import mkdir, system
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
    # project_path.mkdir(parents=True, exist_ok=True)
    # (project_path / "src").mkdir(parents=True, exist_ok=True)
    (project_path / "src" / "_global").mkdir(parents=True, exist_ok=True)

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

deploy_action_str = """
name: Deploy to GitHub Pages

on:
  push:
    branches:
      - main  # 触发条件，推送到 main 分支时执行
  pull_request:
    branches:
      - main  # 触发条件，提交到 main 分支时执行

jobs:
  deploy:
    runs-on: ubuntu-22.04  # 设置运行环境
    permissions:
      contents: write  # 允许写入内容
    concurrency:
      group: ${{ github.workflow }}-${{ github.ref }}  # 避免并发运行

    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          submodules: true  # 如果有子模块需要拉取
          fetch-depth: 0    # 拉取完整历史，以便于获取完整信息（如 .GitInfo）

      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.12'  # 设置 Python 版本，可以根据需要调整

      - name: Install dependencies
        run: |
          python -m pip install --upgrade pip
          pip install blogx=={{VERSION}}  # 安装 blogx

      - name: Build the blog
        run: |
          blogx build  # 运行 blogx 构建静态文件

      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v4
        if: github.ref == 'refs/heads/main'  # 仅在 main 分支执行
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}  # GitHub token
          publish_dir: ./dist  # 指向生成的静态文件目录
""".replace("{{VERSION}}", VERSION)

upload_action_str = """
name: Upload to local server

on:
  workflow_run:
    workflows: ["Deploy to GitHub Pages"]  # 监听 deploy.yml
    types:
      - completed  # 仅在工作流完成时触发

jobs:
    upload:
        runs-on: ubuntu-22.04  # 设置运行环境
        permissions:
            contents: write  # 允许写入内容
        concurrency:
            group: ${{ github.workflow }}-${{ github.ref }}  # 避免并发运行

        if: ${{ github.event.workflow_run.conclusion == 'success' && vars.UPLOAD_ENABLED == 'true' }}  # 添加条件

        steps:
            - name: Checkout repository
              uses: actions/checkout@v4
              with:
                ref: gh-pages

            - name: Install rsync and sshpass
              run: sudo apt-get install rsync sshpass -y

            - name: show current directory
              run: pwd && ls -al

            - name: Upload files
              run: |
                rsync -avz --delete -e "sshpass -p '${{ secrets.SERVER_SSH_PASSWORD }}' ssh -o StrictHostKeyChecking=no" ./ ${{ secrets.SERVER_USER }}@${{ secrets.SERVER_IP }}:${{ secrets.SERVER_WWWROOT_DIR }}
"""

@app.command()
def deploy():
    """Deploy the site to GitHub Pages"""
    is_valid_project()
    # 创建父目录
    if not (current_path / ".github/workflows").exists():
        (current_path / ".github/workflows").mkdir(parents=True, exist_ok=True)
    with open(current_path / ".github/workflows/deploy.yml", "w", encoding='utf-8') as f:
        f.write(deploy_action_str)
    with open(current_path / ".github/workflows/upload.yml", "w", encoding='utf-8') as f:
        f.write(upload_action_str)
    print("Deploy action created. [bold red]DO YOU WANT TO PUSH TO GITHUB?[/bold red]")
    push = input("Push to GitHub? [y/n]: ")
    if push.lower() == "y":
        system("git add .")
        system('git commit -m "Deploy action"')
        system("git push")
        print("[bold green]Pushed to GitHub[/bold green]")

@app.command()
def help(
    command: str = Argument(None, help="Command to get help for"),
):
    if command:
        print(f"Help for {command}")
    else:
        print("Help for the CLI v{VERSION}")

if __name__ == "__main__":
    app()