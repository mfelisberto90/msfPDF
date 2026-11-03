# Third-Party Notices & Font Outline Attribution

## Multilingual Vector Glyph Outlines (`glyphs_data.rs` / `glyphsData.ts`)

`msfPDF` remains a zero-runtime-dependency `%PDF-1.4` engine under the MIT Licence.
Standard Latin & Western European text (`ASCII` + `CP1252` `/WinAnsiEncoding`) uses the built-in PDF Standard 14 Type 1 font metrics (`Helvetica`, `Helvetica-Bold`, `Courier`) with zero embedded font data.

For **Cyrillic (Russian & Slavic)** and **Connected Arabic (RTL + Contextual Presentation Forms)** scripts, `msfPDF` compiles compact cubic Bézier vector outlines into dynamic Type 3 font subsets (`/MsfVectorRegular` and `/MsfVectorBold`, `2048` UPM) derived from the **DejaVu Sans** typeface family:

- **Cyrillic & Arabic Glyph Additions (DejaVu Fonts Project / Arabeyes):** Released into the **Public Domain** by the DejaVu contributors (`DejaVu changes are in public domain`).
- **Base Typeface Notice (Bitstream Vera):**
  Copyright (c) 2003 by Bitstream, Inc. All Rights Reserved. Bitstream Vera is a trademark of Bitstream, Inc.
  Permission is hereby granted, free of charge, to any person obtaining a copy of the fonts accompanying this license ("Fonts") and associated documentation files (the "Font Software"), to reproduce and distribute the Font Software, including without limitation the rights to use, copy, merge, publish, distribute, and/or sell copies of the Font Software, and to permit persons to whom the Font Software is furnished to do so, subject to the following conditions:
  The above copyright and trademark notices and this permission notice shall be included in all copies of one or more of the Font Software typefaces.
  The Font Software may be modified, altered, or added to, and in particular the designs of glyphs or characters in the Fonts may be modified and additional glyphs or characters may be added to the Fonts, only if the fonts are renamed to names not containing either the words "Bitstream" or the word "Vera".
  THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT.
