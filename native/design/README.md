# Design handoff

The desktop screen is a drawing workbench: controls on the left, the result and paid-save decision on the right. The reference art is shown only in the empty state. Once generation completes, the real preview replaces it. The Keep button is unavailable until the preview image has loaded, a finite price is known, and the user confirms that price. Language switching leaves the prompt, photos, selected provider and result intact.

## Hallmark

Applied skill: `C:/Users/Andrey/.codex/plugins/cache/hallmark/hallmark/2.0.0/hallmark/SKILL.md`, app-screen scope. Existing source/palette and user font inspected; official homepage and character page checked. The supplied theme direction was Chiikawa, so no unrelated catalog theme was substituted. Scaffold, tokens, loading/empty/error/full states, visible focus, reduced motion, three locales and narrow-window behavior are included.

Render sweep: `native/design/hallmark-sweep.txt`. Screenshots: `.hallmark/shots/`. Command uses `sloplint.mjs native/ui/index.html native/ui/styles.css native/ui/tokens.css native/ui/base.css --scope app --genre playful --render --url http://127.0.0.1:4176/?demo=1`. The preview URL is only a development server; desktop builds bundle the same files and require no server. Demo mode is labelled and cannot generate images.

## Icon

Applied skill: `C:/Users/Andrey/.codex/plugins/cache/logo-design-skill/logo-design/1.4.5/skills/logo-design/SKILL.md`, Assets mode because the user specified an existing Chiikawa face. Internet reference: https://www.chiikawaofficial.com/characters. The adaptation retains round ears, the face silhouette, highlighted oval eyes, small mouth and hatched pink cheeks. SVG is a stroke-based app asset, not a cutting/engraving master. Four colors include the pink tile and blush, cream face and dark ink.

Master: `native/shared/assets/icon.svg`; inspected 1024px raster: `icon-1024.png`; platform export source: same raster. The actual transparent corners should be preserved in desktop icon export. The Android app uses platform masking. SVG audit reports 98/100, no errors, with informational stroke and tile notices. Exported square/app/favicon variants live in `native/design/icon-variants`. Automatic monochrome recoloring flattened the face details, so those unused variants were discarded; the app ships the tested full-color icon.

## Fonts and attribution

Custom font is used on the wordmark and headings. Native system fonts provide readable controls and Japanese fallback. All shipped artwork in the UI is supplied or traceable to the official character references, not an invented portfolio result. No placeholder image slots remain. Original characters/reference art © nagano / chiikawa committee. This application is unofficial.

## Final review

Independent app-scope finish review requested two fixes: readable character/background selections at 375px and localized accessible names. Both are resolved and visually checked in the final narrow-window capture. The rendered sweep has 0 FAIL, 0 REFLEX and 0 WARN. Browser integration tests cover inspiration, three locales, provider panels, two-photo full transformation, visible preview without automatic payment, cancel/confirm Keep, Codex results, task restoration and persisted language. See native/verification/frontend-tests.json.
