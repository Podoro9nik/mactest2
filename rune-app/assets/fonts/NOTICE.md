# Bundled handwriting fonts

`ChiikawaKawaii-Regular.ttf` is the supplied font, copied without changing its
outlines, names, or license metadata. Its embedded family is **Chiikawa Kawaii**.
It supplies 177 mapped characters, including Latin and Russian Cyrillic. The
source's own copyright notice remains in the font. This file is not relabeled
as an OFL font.

`ChiikawaKawaiiJP-Regular.ttf` is a custom Japanese companion, with family
**Chiikawa Kawaii JP**. It is a renamed derivative of
[Yomogi](https://github.com/satsuyako/YomogiFont), obtained from the
[Google Fonts source repository](https://github.com/google/fonts/tree/main/ofl/yomogi).
The derivative retains the source's 8,039 mapped characters. Its outlines have
rounded expansion, its letter spacing is compacted, and its vertical metrics
match the supplied face. It is an adaptation for this application, not an
official Chiikawa font or an endorsement by the Yomogi authors.

The Japanese companion and its source are licensed under **SIL Open Font
License 1.1**, reproduced in `OFL-Yomogi.txt`. The original Yomogi copyright
notice is preserved. No outlines from the separately supplied Chiikawa font
are merged into the OFL derivative.

Use the ordered family list `Chiikawa Kawaii, Chiikawa Kawaii JP` for both Rune
font slots. The supplied Latin/Cyrillic shapes then take priority in every
language, while Japanese and additional punctuation come from the companion.
Do not select an operating-system UI font for app labels, inputs, dialogs,
status text, or buttons. The Windows installer can select the original face
for Russian/English pages and the Japanese companion for Japanese pages.

`font-proof.png` is a rendered comparison at heading and control sizes;
`font-report.json` records coverage and the final checksum. Rebuilding uses
`tools/rune-fonts/build_japanese.py`; Python is a development tool only and is
not needed by the packaged Rune application.
