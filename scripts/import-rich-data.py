#!/usr/bin/env python3
"""Development-only: import MIT-licensed Rich 12.6 name tables into static assets.

Run with the isolated reference Python: python scripts/import-rich-data.py
The shipped Rust executable does not use or invoke Python.
"""
import json
from pathlib import Path
from rich.color import ANSI_COLOR_NAMES
from rich._emoji_codes import EMOJI

root = Path(__file__).resolve().parents[1] / 'assets'
root.mkdir(exist_ok=True)
for name, values in [('colors', ANSI_COLOR_NAMES), ('emoji', EMOJI)]:
    (root / f'{name}.json').write_text(json.dumps(values, ensure_ascii=False, sort_keys=True, indent=2)+'\n')

# Pygments and TextMate use different token taxonomies. These palettes preserve
# theme colors, while scope selection is an approximation documented in COMPATIBILITY.md.
import plistlib
from pygments.styles import get_style_by_name
from pygments.token import Token
scopes = [
    (Token.Comment, 'comment'),
    (Token.Comment.Preproc, 'comment.block.preprocessor'),
    (Token.Keyword, 'keyword'),
    (Token.Keyword.Type, 'storage.type'),
    (Token.Keyword.Declaration, 'storage.modifier'),
    (Token.Name.Function, 'entity.name.function'),
    (Token.Name.Class, 'entity.name.class'),
    (Token.Name.Builtin, 'support.function'),
    (Token.Name.Decorator, 'entity.name.function.decorator'),
    (Token.Literal.String, 'string'),
    (Token.Literal.String.Escape, 'constant.character.escape'),
    (Token.Literal.Number, 'constant.numeric'),
    (Token.Operator, 'keyword.operator'),
    (Token.Punctuation, 'punctuation'),
    (Token.Generic.Heading, 'markup.heading'),
    (Token.Generic.Strong, 'markup.bold'),
    (Token.Generic.Emph, 'markup.italic'),
    (Token.Error, 'invalid'),
]
(root/'themes').mkdir(exist_ok=True)
for name in ['monokai','dracula','default','friendly']:
    theme=get_style_by_name(name)
    settings=[{'settings': {'foreground':'#'+(theme.style_for_token(Token.Text)['color'] or 'eeeeee'),
                            'background': theme.background_color}}]
    for token, scope in scopes:
        style=theme.style_for_token(token)
        values={}
        if style['color']:
            values['foreground']='#'+style['color']
        values['fontStyle']=' '.join(k for k in ['bold','italic','underline'] if style[k])
        settings.append({'scope':scope,'settings':values})
    (root/'themes'/f'{name}.tmTheme').write_bytes(plistlib.dumps({'name':name,'settings':settings}))
