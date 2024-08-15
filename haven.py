import markdown
import re
from markdown_katex import KatexExtension
from markdown.extensions.codehilite import CodeHiliteExtension
import os, shutil

template_path = os.path.join("themes", 'haven', 'template.html')
with open(template_path, 'r', encoding='utf-8') as f:
    template = f.read()

def md2html(md_content):
    # 将$...$替换为$`...`$，$$...$$替换为```math...```，\(...\)替换为$`...`$，\[...]替换为```math...```，以支持 KatexExtension
    md_content = re.sub(r'\$\$(.*?)\$\$', r'```math\1```', md_content, flags=re.DOTALL)  # 块级公式
    md_content = re.sub(r'\$(.*?)\$', r'$`\1`$', md_content)  # 行内公式
    md_content = re.sub(r'\\\((.*?)\\\)', r'$`\1`$', md_content)  # 行内公式
    md_content = re.sub(r'\\\[(.*?)\\\]', r'```math\1```', md_content, flags=re.DOTALL)  # 块级公式
    # 将 Markdown 转换为 HTML，并启用 codehilite、fenced_code、KatexExtension 扩展
    html_content = markdown.markdown(md_content, extensions=[ 
        'fenced_code', 
        'tables', 
        KatexExtension(insert_fonts_css=False), 
        CodeHiliteExtension(linenos=True)])
    return html_content

def render(md_content):
    html_content = md2html(md_content)
    return template.replace('{{ article_content|safe }}', html_content)

if __name__ == '__main__':
    # 删除dist文件夹
    if os.path.exists('dist'):
        shutil.rmtree('dist')
    # 创建dist文件夹
    os.mkdir('dist')
    # 遍历blog文件夹，在dist下创建相同的文件夹结构
    for root, dirs, files in os.walk('blog'):
        root = root.replace('blog', 'dist')
        for dir in dirs:
            os.mkdir(os.path.join(root, dir))
    # 遍历blog文件夹，将md文件转换为html文件
    for root, dirs, files in os.walk('blog'):
        for file in files:
            if file.endswith('.md'):
                with open(os.path.join(root, file), 'r', encoding='utf-8') as f:
                    md_content = f.read()
                html_content = render(md_content)
                with open(os.path.join(root.replace('blog', 'dist'), file.replace('.md', '.html')), 'w', encoding='utf-8') as f:
                    f.write(html_content)
    # 复制static文件夹到dist文件夹
    shutil.copytree('themes/haven/static', 'dist/static')