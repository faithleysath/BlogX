from flask import Flask, render_template, request
import markdown
import re
from markdown_katex import KatexExtension
from markdown.extensions.codehilite import CodeHiliteExtension

app = Flask(__name__)

def md2html(md_content):
    # 替换公式
    md_content = re.sub(r'\$\$(.*?)\$\$', r'```math\1```', md_content, flags=re.DOTALL)  # 块级公式
    md_content = re.sub(r'\$(.*?)\$', r'$`\1`$', md_content)  # 行内公式
    md_content = re.sub(r'\\\((.*?)\\\)', r'$`\1`$', md_content)  # 行内公式
    md_content = re.sub(r'\\\[(.*?)\\\]', r'```math\1```', md_content, flags=re.DOTALL)  # 块级公式
    # 将 Markdown 转换为 HTML
    html_content = markdown.markdown(md_content, extensions=[ 
        'fenced_code', 
        'tables', 
        KatexExtension(insert_fonts_css=False), 
        CodeHiliteExtension(linenos=True)
    ])
    return html_content

@app.route('/')
def index():
    return render_template('template.html', article_content='欢迎来到我的博客！')

@app.route('/render', methods=['POST'])
def render_markdown():
    md_content = request.form['markdown']
    html_content = md2html(md_content)
    return render_template('template.html', article_content=html_content)

if __name__ == '__main__':
    app.run(debug=True)
