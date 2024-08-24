from pathlib import Path

root_path = Path(__file__).parent
themes_path = root_path / "themes"

themes = [theme.name for theme in themes_path.iterdir() if theme.is_dir()]
