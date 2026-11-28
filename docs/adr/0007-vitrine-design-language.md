# "Vitrine" design language: the interface steps back, covers are the only colour

Covers are the strongest visual material a game library has, and they bring their own colours. The interface is therefore neutral graphite with no brand accent colour at all; a game's detail page takes its tint and accent from that game's Cover. Layout is a fixed sidebar (navigation and Collections) with a cover grid as the main surface and the game detail as its own page.

Both were chosen from throwaway prototypes on the branch `prototype/ui-variants` (`?variant=` for layout, `?look=` for the visual language).

## Considered Options

- **Layout B (storefront with shelves and a hero banner)** and **layout C (dense list with a permanent detail pane)**: rejected in favour of A, sidebar plus cover grid.
- **"Hülle"** (covers as physical cases on shelf boards, status stickers, amber accent): distinctive, but stickers cover part of every Cover and the spine view has little room for information.
- **"Konsole"** (navy colour field, large radii, travelling selection frame, mint accent): playful, but the wide display face costs space on long titles.
- **The round-1 default look** (violet accent, system font, rounded everything): it was what the model produces without direction, not a choice.

## The language

- **Colour**: surfaces `#121314` dark / `#F2F2F0` light, text `#ECEBE8` / `#1A1B1C`. The "accent" in the library is the text colour itself (inverted primary button, white progress line). On a detail page, accent and background tint derive from the Cover's dominant hue. Status colours (playing, completed, dropped) are the only other colours.
- **Brand mark (exception)**: the "nl" monogram, a lime gradient `#E4FF7A` → `#7FD66B` on the dark plate `#121314`, is the app icon and the mark beside the wordmark in the sidebar (#92). It is the first exception to "no brand accent colour at all": the mark carries colour, the rest of the interface stays neutral. The mark keeps its dark plate in both themes, so it looks the same in dark and light.
- **Type**: Bricolage Grotesque for display (page titles, game titles, key figures), Instrument Sans for everything else. Both OFL, shipped as files with the app, never from a CDN.
- **Shape**: small radii by role (Covers 3px, controls 4px), no pills, no cards: sections are separated by hairlines, not boxed.
- **Covers**: shown large, uncropped, unframed. Games that are not Installed are desaturated.
- **Labels**: sentence case, no letter-spaced capitals.

## Consequences

- With real Covers the tint has to be extracted from the image; the prototype only had a known hue per placeholder. Contrast of the derived accent must be checked for every Cover, with a neutral fallback.
- Raw colour values live in one token file. Components use tokens only.
- The prototype code is not promoted; the real implementation is written fresh against this ADR.
