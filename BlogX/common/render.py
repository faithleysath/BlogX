import markdown
import re
from markdown_katex import KatexExtension
from markdown.extensions.codehilite import CodeHiliteExtension

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
    # 将所有img标签加上referrerpolicy="no-referrer"属性
    html_content = re.sub(r'<img', r'<img referrerpolicy="no-referrer"', html_content)
    return html_content
