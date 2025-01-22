import markdown
import re
from markdown_katex import KatexExtension
from markdown.extensions.codehilite import CodeHiliteExtension
import base64

def encode_to_base64(input_string):
    # 将字符串转换为字节
    input_bytes = input_string.encode('utf-8')
    
    # 使用 base64 编码
    encoded_bytes = base64.b64encode(input_bytes)
    
    # 将编码后的字节转换回字符串
    encoded_string = encoded_bytes.decode('utf-8')
    
    return encoded_string

def md2html(md_content):
    # 将$...$替换为$`...`$，$$...$$替换为```math...```，\(...\)替换为$`...`$，\[...]替换为```math...```，以支持 KatexExtension
    md_content = re.sub(r'\$\$(.*?)\$\$', r'```math\1```', md_content, flags=re.DOTALL)  # 块级公式
    md_content = re.sub(r'\$(.*?)\$', r'$`\1`$', md_content)  # 行内公式
    md_content = re.sub(r'\\\((.*?)\\\)', r'$`\1`$', md_content)  # 行内公式
    md_content = re.sub(r'\\\[(.*?)\\\]', r'```math\1```', md_content, flags=re.DOTALL)  # 块级公式
    # 将markdown里的超链接里的.md替换为.html
    md_content = re.sub(r'\((.*?)\.md\)', r'(\1.html)', md_content)
    # 将 Markdown 转换为 HTML，并启用 codehilite、fenced_code、KatexExtension 扩展
    html_content = markdown.markdown(md_content, extensions=[ 
        'fenced_code', 
        'tables', 
        KatexExtension(insert_fonts_css=False), 
        CodeHiliteExtension(linenos=True)])
    # 将所有img标签加上referrerpolicy="no-referrer"属性
    html_content = re.sub(r'<img', r'<img referrerpolicy="no-referrer"', html_content)
    return html_content

from pathlib import Path
import shutil

def get_template(input_dir: Path, theme_dir: Path):
    with open(theme_dir / "template.html", "r", encoding='utf-8') as f:
        template = f.read()
    global_dir = input_dir / "_global"
    with open(global_dir / "BLOGNAME", "r", encoding='utf-8') as f:
        blog_name = f.read()
    with open(global_dir / "header.md", "r", encoding='utf-8') as f:
        header = md2html(f.read())
    with open(global_dir / "footer.md", "r", encoding='utf-8') as f:
        footer = md2html(f.read())
    with open(global_dir / "sidebar.md", "r", encoding='utf-8') as f:
        sidebar = md2html(f.read())
    template = template.replace("{{ title }}", blog_name).replace("{{ header_content|safe }}", header).replace("{{ sidebar_content|safe }}", sidebar).replace("{{ footer_content|safe }}", footer)
    return template

def build_single_file(input_root: Path, output_root: Path, template: str, file: Path):
    # 先建立父目录
    (output_root / file.relative_to(input_root).parent).mkdir(parents=True, exist_ok=True)
    # 如果是md文件，读取并渲染
    if file.suffix == ".md":
        with open(file, "r", encoding='utf-8') as f:
            content = md2html(f.read())
        # 如果需要保护
        if str(file).endswith(".protect.md"):
            file = Path(str(file).replace(".protect", ""))
            file_content = template.replace("{{ enableAllProtections|safe }}", "true").replace("{{ article_content|safe }}", "")
            # 写入html到output_root
            with open(output_root / file.with_suffix(".html").relative_to(input_root), "w", encoding='utf-8') as f:
                f.write(file_content)
            encrypt_content = encode_to_base64(content)
            with open(output_root / file.with_suffix(".html.encrypted").relative_to(input_root), "w", encoding='utf-8') as f:
                f.write(encrypt_content)
        else:
            file_content = template.replace("{{ enableAllProtections|safe }}", "false").replace("{{ article_content|safe }}", content)
            # 写入html到output_root
            with open(output_root / file.with_suffix(".html").relative_to(input_root), "w", encoding='utf-8') as f:
                f.write(file_content)
    # 如果不是md文件，直接复制
    else:
        shutil.copy(file, output_root / file.relative_to(input_root))

def build_site(input_dir: Path, output_dir: Path, theme_dir: Path):
    if output_dir.exists():
        shutil.rmtree(output_dir)
    shutil.copytree(theme_dir, output_dir)
    # 获取模板
    template = get_template(input_dir, theme_dir)
    # 处理input_dir里除_global外的所有文件（包括子目录）
    for file in input_dir.rglob("*"):
        if file.is_file() and "_global" not in file.parts:
            build_single_file(input_dir, output_dir, template, file)
    # 将_global/favicon.ico复制到output_dir/favicon.ico
    if (input_dir / "_global/favicon.ico").exists():
        shutil.copy(input_dir / "_global/favicon.ico", output_dir / "favicon.ico")

from pathlib import Path
from logging import getLogger
from livereload import Server

def dev_server(input_dir: Path, output_dir: Path, theme_dir: Path):
    def src_on_modified(change_files=None):
        getLogger("watcher").info(f"Detected src change in {change_files}")
        if change_files is None:
            build_site(input_dir, output_dir, theme_dir)
            return
        template = get_template(input_dir, theme_dir)
        for file in change_files:
            if "_global" in file:
                build_site(input_dir, output_dir, theme_dir)
                return
            build_single_file(input_dir, output_dir, template, Path(file))

    def theme_on_modified(change_files=None):
        getLogger("watcher").info(f"Detected theme change in {change_files}")
        build_site(input_dir, output_dir, theme_dir)

    # first build
    build_site(input_dir, output_dir, theme_dir)
    # create a server instance
    server = Server()
    # watch the src directory
    server.watch(input_dir, src_on_modified)
    # watch the theme directory
    server.watch(theme_dir, theme_on_modified)
    # start the server
    server.serve(root=output_dir)