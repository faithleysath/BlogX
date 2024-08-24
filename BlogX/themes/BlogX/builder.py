from pathlib import Path
from BlogX.common.render import md2html
from functools import lru_cache
from shutil import copytree
import logging

@lru_cache(maxsize=None)
def get_sidebar(sidebar_path: Path):
    with open(sidebar_path, 'r', encoding='utf-8') as f:
        return md2html(f.read())
    
def render(md_content: str, sidebar: str, template: str):
    return template.replace('{{ article_content|safe }}', md2html(md_content)).replace('{{ sidebar_content|safe }}', sidebar)

def build(input_path: Path, output_path: Path, specific_file: Path = None):
    with open(Path(__file__).parent / 'template.html', 'r', encoding='utf-8') as f:
        template = f.read()
    if specific_file:
        if specific_file.suffix == '.md' and specific_file.name != 'sidebar.md':
            logging.info(f"Building {specific_file}")
            specific_file_rel = specific_file.relative_to(input_path)
            output_file = output_path / specific_file_rel.with_suffix('.html')
            with open(specific_file, 'r', encoding='utf-8') as f:
                md_content = f.read()
            with open(output_file, 'w', encoding='utf-8') as f:
                f.write(render(md_content, get_sidebar(input_path / 'sidebar.md'), template))
            return
    logging.info("Building the entire site")
    get_sidebar.cache_clear()
    # copy all files from input_path to output_path
    copytree(input_path, output_path, dirs_exist_ok=True)
    get_sidebar(output_path / 'sidebar.md')
    # convert all .md files to .html files except sidebar.md
    for md_file in output_path.rglob('*.md'):
        if md_file.name != 'sidebar.md':
            html_file = md_file.with_suffix('.html')
            with open(md_file, 'r', encoding='utf-8') as f:
                md_content = f.read()
            with open(html_file, 'w', encoding='utf-8') as f:
                f.write(render(md_content, get_sidebar(output_path / 'sidebar.md'), template))
        md_file.unlink()
    # copy the static folder
    copytree(Path(__file__).parent / 'static', output_path / 'static', dirs_exist_ok=True)