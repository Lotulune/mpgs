import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GameMediaBlock } from "../src/api/types";
import {
  buildGalleryItems,
  GameMediaGallery,
} from "../src/components/GameMediaGallery";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const fullMedia: GameMediaBlock = {
  updated_at_ms: 1_700_000_000_000,
  screenshots: [
    {
      id: "0",
      thumbnail_url: "https://shared.akamai.steamstatic.com/t0.jpg",
      full_url: "https://shared.akamai.steamstatic.com/f0.jpg",
    },
    {
      id: "1",
      thumbnail_url: "https://shared.akamai.steamstatic.com/t1.jpg",
      full_url: "https://shared.akamai.steamstatic.com/f1.jpg",
    },
  ],
  videos: [
    {
      id: "v1",
      title: "Highlight Trailer",
      poster_url: "https://shared.akamai.steamstatic.com/p1.jpg",
      highlight: true,
      mp4_url: "https://video.akamai.steamstatic.com/a.mp4",
      hls_h264_url: null,
      dash_h264_url: null,
    },
    {
      id: "v2",
      title: "Other Trailer",
      poster_url: "https://shared.akamai.steamstatic.com/p2.jpg",
      highlight: false,
      mp4_url: null,
      hls_h264_url: "https://video.akamai.steamstatic.com/b.m3u8",
      dash_h264_url: null,
    },
  ],
};

function mountGallery(props: {
  coverUrl?: string | null;
  media?: GameMediaBlock | null;
  steamUrl?: string;
}) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => {
    root.render(
      <GameMediaGallery
        appId={892970}
        name="Valheim"
        coverUrl={props.coverUrl ?? "https://shared.akamai.steamstatic.com/header.jpg"}
        media={props.media}
        steamUrl={props.steamUrl ?? "https://store.steampowered.com/app/892970/"}
      />,
    );
  });
  return {
    host,
    root,
    unmount() {
      act(() => root.unmount());
      host.remove();
    },
  };
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("buildGalleryItems", () => {
  it("orders cover, highlight videos, screenshots, then other videos", () => {
    const items = buildGalleryItems({
      appId: 892970,
      name: "Valheim",
      coverUrl: "https://shared.akamai.steamstatic.com/header.jpg",
      media: fullMedia,
    });
    expect(items.map((i) => i.id)).toEqual([
      "cover",
      "video-v1",
      "shot-0",
      "shot-1",
      "video-v2",
    ]);
  });

  it("treats missing media as empty and still shows cover", () => {
    const items = buildGalleryItems({
      appId: 892970,
      name: "Valheim",
      coverUrl: "https://shared.akamai.steamstatic.com/header.jpg",
      media: undefined,
    });
    expect(items).toHaveLength(1);
    expect(items[0]?.kind).toBe("cover");
  });

  it("dedupes cover URL against identical screenshot full URL", () => {
    const items = buildGalleryItems({
      appId: 1,
      name: "Game",
      coverUrl: "https://shared.akamai.steamstatic.com/same.jpg",
      media: {
        updated_at_ms: 1,
        screenshots: [
          {
            id: "9",
            thumbnail_url: "https://shared.akamai.steamstatic.com/same-t.jpg",
            full_url: "https://shared.akamai.steamstatic.com/same.jpg",
          },
        ],
        videos: [],
      },
    });
    expect(items.filter((i) => i.kind === "screenshot")).toHaveLength(0);
    expect(items[0]?.kind).toBe("cover");
  });
});

describe("GameMediaGallery", () => {
  it("renders only cover when media is missing (old server)", () => {
    const { host, unmount } = mountGallery({ media: undefined });
    expect(host.querySelector('[data-testid="game-media-gallery"]')).toBeTruthy();
    expect(host.querySelectorAll(".gallery-thumb")).toHaveLength(0);
    expect(host.querySelector(".gallery-stage-image img")?.getAttribute("src")).toContain(
      "header.jpg",
    );
    unmount();
  });

  it("renders empty media arrays as cover-only experience", () => {
    const { host, unmount } = mountGallery({
      media: { updated_at_ms: null, screenshots: [], videos: [] },
    });
    expect(host.querySelectorAll(".gallery-thumb")).toHaveLength(0);
    unmount();
  });

  it("supports click and keyboard thumbnail switching with accessible current state", () => {
    const { host, unmount } = mountGallery({ media: fullMedia });
    const thumbs = host.querySelectorAll<HTMLButtonElement>(".gallery-thumb");
    expect(thumbs.length).toBeGreaterThan(1);
    expect(thumbs[0]?.getAttribute("aria-selected")).toBe("true");
    expect(thumbs[0]?.getAttribute("aria-current")).toBe("true");

    act(() => {
      thumbs[2]?.click();
    });
    expect(thumbs[2]?.getAttribute("aria-selected")).toBe("true");

    act(() => {
      host.querySelector(".gallery-rail")!.dispatchEvent(
        new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true }),
      );
    });
    expect(thumbs[3]?.getAttribute("aria-selected")).toBe("true");

    act(() => {
      host.querySelector(".gallery-rail")!.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Home", bubbles: true, cancelable: true }),
      );
    });
    expect(thumbs[0]?.getAttribute("aria-selected")).toBe("true");

    act(() => {
      host.querySelector(".gallery-rail")!.dispatchEvent(
        new KeyboardEvent("keydown", { key: "End", bubbles: true, cancelable: true }),
      );
    });
    expect(thumbs[thumbs.length - 1]?.getAttribute("aria-selected")).toBe("true");
    unmount();
  });

  it("skips failed images without breaking the gallery", () => {
    const { host, unmount } = mountGallery({ media: fullMedia });
    const mainImg = host.querySelector<HTMLImageElement>(".gallery-stage-image img")!;
    act(() => {
      mainImg.dispatchEvent(new Event("error"));
    });
    // After cover fails, next item becomes active (highlight video).
    expect(host.querySelector(".gallery-stage-video")).toBeTruthy();
    unmount();
  });

  it("does not load video sources until the user clicks play", () => {
    const { host, unmount } = mountGallery({
      media: {
        updated_at_ms: 1,
        screenshots: [],
        videos: [
          {
            id: "only",
            title: "Trailer",
            poster_url: "https://shared.akamai.steamstatic.com/p.jpg",
            highlight: true,
            mp4_url: "https://video.akamai.steamstatic.com/only.mp4",
            hls_h264_url: null,
            dash_h264_url: null,
          },
        ],
      },
      coverUrl: null,
    });
    // Without a cover, highlight video is first; no src until play.
    // Force select video if cover CDN candidates exist from appId.
    const videoThumb = Array.from(host.querySelectorAll<HTMLButtonElement>(".gallery-thumb")).find(
      (btn) => btn.classList.contains("is-video"),
    );
    if (videoThumb) {
      act(() => videoThumb.click());
    }
    const video = host.querySelector<HTMLVideoElement>(".gallery-video")!;
    expect(video.getAttribute("src")).toBeNull();
    expect(video.autoplay).toBe(false);
    expect(host.querySelector(".gallery-play-btn")).toBeTruthy();
    unmount();
  });

  it("shows Steam fallback when playback cannot start", async () => {
    const playSpy = vi
      .spyOn(HTMLMediaElement.prototype, "play")
      .mockRejectedValue(new Error("not allowed"));
    const { host, unmount } = mountGallery({
      media: {
        updated_at_ms: 1,
        screenshots: [],
        videos: [
          {
            id: "only",
            title: "Trailer",
            poster_url: "https://shared.akamai.steamstatic.com/p.jpg",
            highlight: true,
            mp4_url: "https://video.akamai.steamstatic.com/only.mp4",
            hls_h264_url: null,
            dash_h264_url: null,
          },
        ],
      },
    });
    const videoThumb = Array.from(host.querySelectorAll<HTMLButtonElement>(".gallery-thumb")).find(
      (btn) => btn.classList.contains("is-video"),
    );
    if (videoThumb) {
      act(() => videoThumb.click());
    }
    await act(async () => {
      host.querySelector<HTMLButtonElement>(".gallery-play-btn")?.click();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(host.textContent).toContain("当前环境无法播放预告片");
    expect(host.querySelector('a[href*="store.steampowered.com"]')).toBeTruthy();
    playSpy.mockRestore();
    unmount();
  });

  it("opens lightbox for images and Escape closes without bubbling to the page", () => {
    const { host, unmount } = mountGallery({ media: fullMedia });
    const underlying = vi.fn();
    window.addEventListener("keydown", underlying);

    act(() => {
      host.querySelector<HTMLButtonElement>(".gallery-stage-image")?.click();
    });
    expect(host.querySelector(".gallery-lightbox")).toBeTruthy();
    expect(document.querySelector('[role="dialog"][aria-modal="true"]')).toBeTruthy();

    const dialog = document.querySelector<HTMLElement>('[role="dialog"]')!;
    act(() => {
      dialog.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }),
      );
    });
    // Modal stops propagation; page handler must not run for the dialog key path.
    expect(underlying).not.toHaveBeenCalled();

    window.removeEventListener("keydown", underlying);
    unmount();
  });
});
