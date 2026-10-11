import { afterEach, describe, expect, it } from "vitest";

import {
  activeCardArtSource,
  applyCardArtSource,
  MTGCH_IMAGE_HOST,
  setActiveCardArtSourceForTest,
} from "../cardArtSource.ts";
import { usePreferencesStore } from "../../stores/preferencesStore.ts";

/**
 * Realistic Scryfall face URLs. The five-segment shape is load-bearing: both
 * `splitSizedImageUrl` and `applyCardArtSource` discriminate on segment count, so
 * a test using a short stand-in URL would pass while the real pipeline failed.
 * LEA 161 Lightning Bolt, the printing every value below was verified against.
 */
const PRINTING_ID = "d573ef03-4730-45aa-93dd-e45ac1dbaf4a";
const scryfallUrl = (size: string, face: "front" | "back" = "front", query = "") =>
  `https://cards.scryfall.io/${size}/${face}/d/5/${PRINTING_ID}.jpg${query}`;

const mtgchUrl = (size: string, face: "front" | "back" = "front") =>
  `https://${MTGCH_IMAGE_HOST}/zhs/${size}/${face}/d/5/${PRINTING_ID}.webp`;

afterEach(() => {
  setActiveCardArtSourceForTest(null);
  usePreferencesStore.setState({ chineseCardArt: true, language: "en" });
});

describe("applyCardArtSource", () => {
  it("rewrites a sized Scryfall URL to the same printing's Chinese art", () => {
    expect(applyCardArtSource(scryfallUrl("normal"), "mtgch")).toBe(mtgchUrl("normal"));
    expect(applyCardArtSource(scryfallUrl("small"), "mtgch")).toBe(mtgchUrl("small"));
    expect(applyCardArtSource(scryfallUrl("large"), "mtgch")).toBe(mtgchUrl("large"));
  });

  it("preserves the face, so a DFC back face keeps its own Chinese art", () => {
    expect(applyCardArtSource(scryfallUrl("normal", "back"), "mtgch"))
      .toBe(mtgchUrl("normal", "back"));
  });

  it("drops the English printing's cache-busting query", () => {
    // `?1783905318` is a cache-buster for the *Scryfall* object and means nothing
    // for a different host's asset. Carrying it over would also defeat the CDN's
    // immutable caching for the Chinese URL.
    expect(applyCardArtSource(scryfallUrl("normal", "front", "?1783905318"), "mtgch"))
      .toBe(mtgchUrl("normal"));
  });

  it("leaves art_crop alone because mtgch publishes no Chinese crop", () => {
    // Verified live: images.mtgch.com/zhs/art_crop/... is a hard 404 while the
    // English /sf/art_crop/ path returns 200. Synthesizing the URL would break
    // the board, whose default display IS art_crop.
    const crop = scryfallUrl("art_crop");
    expect(applyCardArtSource(crop, "mtgch")).toBe(crop);
  });

  it("passes through everything it cannot serve, byte-identically", () => {
    // Byte-identity matters beyond tidiness: `isPlaceholderImageUrl` gates the
    // printing-fallback chain with `===`, so any rewrite of the placeholder
    // silently disables art fallback for every card with missing art.
    const untouched = [
      "https://errors.scryfall.com/soon.jpg",
      "https://backs.scryfall.io/normal/0/a/0aeebaf5-8c7d-4636-9e82-8c27447861f7.jpg",
      "https://cards.scryfall.io/normal/front/m/a/manifest.jpg",
      "",
      "not a url",
      // Already rewritten: a second pass must be a no-op, not a doubled rewrite.
      mtgchUrl("normal"),
    ];
    for (const url of untouched) {
      expect(applyCardArtSource(url, "mtgch"), url).toBe(url);
    }
  });

  it("is a no-op for the scryfall source", () => {
    expect(applyCardArtSource(scryfallUrl("normal"), "scryfall")).toBe(scryfallUrl("normal"));
  });

  it("does not depend on the host, only on the path shape", () => {
    // Test fixtures and the visual-pack backend emit stand-in hosts; the
    // rewrite must still fire so those paths are exercisable offline.
    expect(applyCardArtSource(`https://example.test/normal/front/d/5/${PRINTING_ID}.jpg`, "mtgch"))
      .toBe(mtgchUrl("normal"));
  });
});

describe("activeCardArtSource", () => {
  it("uses Chinese art only when the UI language is Chinese and the switch is on", () => {
    setActiveCardArtSourceForTest(null);

    usePreferencesStore.setState({ language: "zh", chineseCardArt: true });
    expect(activeCardArtSource()).toBe("mtgch");

    usePreferencesStore.setState({ chineseCardArt: false });
    expect(activeCardArtSource()).toBe("scryfall");

    // The switch defaults to ON, so keying off it alone would hand Chinese art
    // to a player whose UI is English — a preference they never set.
    usePreferencesStore.setState({ chineseCardArt: true, language: "en" });
    expect(activeCardArtSource()).toBe("scryfall");

    usePreferencesStore.setState({ language: "de" });
    expect(activeCardArtSource()).toBe("scryfall");
  });

  it("lets a test pin the source and restore it without leaking", () => {
    usePreferencesStore.setState({ language: "en", chineseCardArt: false });
    setActiveCardArtSourceForTest("mtgch");
    expect(activeCardArtSource()).toBe("mtgch");
    setActiveCardArtSourceForTest(null);
    expect(activeCardArtSource()).toBe("scryfall");
  });
});
