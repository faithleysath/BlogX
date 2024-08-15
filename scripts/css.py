from pygments import highlight
from pygments.lexers import PythonLexer  # 选择你需要的语言
from pygments.formatters import HtmlFormatter

# 选择你想要的样式
style_name = 'lightbulb'  # 或者其他样式，如 'default', 'friendly', 等

# 创建一个 HtmlFormatter 对象，指定样式和其他选项
formatter = HtmlFormatter(style=style_name, full=True, linenos='table')

# 生成 CSS
css = formatter.get_style_defs()

# 输出到文件
with open('pygments.css', 'w') as f:
    f.write(css)

print(f'pygments.css 文件已生成，使用样式：{style_name}')
