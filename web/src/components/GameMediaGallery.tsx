// Detail-page media gallery: cover + Steam screenshots + trailers.
// List cards keep using GameMedia; this component is detail-only.

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import type { GameMediaBlock, GameMediaScreenshot, GameMediaVideo } from "../api/types";
import { coverCandidates } from "./GameMedia";
import { Modal } from "./Modal";

export type GalleryImageItem = {
  kind: "cover" | "screenshot";
  id: string;
  thumbnailUrl: string;
  fullUrl: string;
  alt: string;
};

export type GalleryVideoItem = {
  kind: "video";
  id: string;
  posterUrl: string;
  title: string;
  mp4Url: string | null;
  hlsUrl: string | null;
  dashUrl: string | null;
};

export type GalleryItem = GalleryImageItem | GalleryVideoItem;

/** Assemble gallery order: cover → highlight videos → screenshots → other videos. */
export function buildGalleryItems(input: {
  appId: number;
  name: string;
  coverUrl: string | null | undefined;
  media: GameMediaBlock | null | undefined;
}): GalleryItem[] {
  const media = input.media;
  const screenshots: GameMediaScreenshot[] = media?.screenshots ?? [];
  const videos: GameMediaVideo[] = media?.videos ?? [];

  const items: GalleryItem[] = [];
  const seenUrls = new Set<string>();

  const coverCandidatesList = coverCandidates(input.appId, input.coverUrl);
  const coverUrl = coverCandidatesList[0];
  if (coverUrl) {
    items.push({
      kind: "cover",
      id: "cover",
      thumbnailUrl: coverUrl,
      fullUrl: coverUrl,
      alt: `${input.name} 封面`,
    });
    seenUrls.add(coverUrl);
  }

  const highlightVideos = videos.filter((v) => v.highlight);
  const otherVideos = videos.filter((v) => !v.highlight);

  const pushVideo = (video: GameMediaVideo) => {
    items.push({
      kind: "video",
      id: `video-${video.id}`,
      posterUrl: video.poster_url,
      title: video.title?.trim() || "预告片",
      mp4Url: video.mp4_url,
      hlsUrl: video.hls_h264_url,
      dashUrl: video.dash_h264_url,
    });
  };

  for (const video of highlightVideos) pushVideo(video);

  for (const shot of screenshots) {
    if (seenUrls.has(shot.full_url) || seenUrls.has(shot.thumbnail_url)) continue;
    seenUrls.add(shot.full_url);
    items.push({
      kind: "screenshot",
      id: `shot-${shot.id}`,
      thumbnailUrl: shot.thumbnail_url,
      fullUrl: shot.full_url,
      alt: `${input.name} 截图`,
    });
  }

  for (const video of otherVideos) pushVideo(video);
  return items;
}

function canPlayNativeHls(video: HTMLVideoElement | null): boolean {
  if (!video) return false;
  // Safari / some WebViews can play HLS natively.
  return video.canPlayType("application/vnd.apple.mpegurl") !== "";
}

type HlsLike = {
  loadSource: (src: string) => void;
  attachMedia: (media: HTMLMediaElement) => void;
  on: (event: string, cb: (...args: unknown[]) => void) => void;
  destroy: () => void;
};

type HlsConstructor = {
  new (config?: { enableWorker?: boolean; autoStartLoad?: boolean }): HlsLike;
  isSupported: () => boolean;
  Events: { ERROR: string };
};

async function loadHlsConstructor(): Promise<HlsConstructor | null> {
  try {
    const mod = await import("hls.js");
    return (mod.default ?? mod) as unknown as HlsConstructor;
  } catch {
    return null;
  }
}

function VideoStage({
  item,
  steamUrl,
  active,
}: {
  item: GalleryVideoItem;
  steamUrl: string;
  active: boolean;
}) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const hlsRef = useRef<HlsLike | null>(null);
  const [playing, setPlaying] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const destroyPlayer = useCallback(() => {
    if (hlsRef.current) {
      hlsRef.current.destroy();
      hlsRef.current = null;
    }
    const el = videoRef.current;
    if (el) {
      try {
        el.pause();
      } catch {
        // jsdom does not implement media pause/load.
      }
      el.removeAttribute("src");
      try {
        el.load();
      } catch {
        // ignore unsupported media APIs in test environments
      }
    }
  }, []);

  useEffect(() => {
    if (!active) {
      destroyPlayer();
      setPlaying(false);
      setError(null);
    }
  }, [active, destroyPlayer]);

  useEffect(() => () => destroyPlayer(), [destroyPlayer]);

  const startPlayback = useCallback(async () => {
    setError(null);
    const el = videoRef.current;
    if (!el) return;

    destroyPlayer();

    if (item.mp4Url) {
      el.src = item.mp4Url;
      setPlaying(true);
      try {
        await el.play();
      } catch {
        setError("当前环境无法播放预告片");
        setPlaying(false);
      }
      return;
    }

    if (item.hlsUrl && canPlayNativeHls(el)) {
      el.src = item.hlsUrl;
      setPlaying(true);
      try {
        await el.play();
      } catch {
        setError("当前环境无法播放预告片");
        setPlaying(false);
      }
      return;
    }

    if (item.hlsUrl) {
      const Hls = await loadHlsConstructor();
      if (Hls && Hls.isSupported()) {
        const hls = new Hls({ enableWorker: true, autoStartLoad: true });
        hlsRef.current = hls;
        hls.loadSource(item.hlsUrl);
        hls.attachMedia(el);
        hls.on(Hls.Events.ERROR, (...args: unknown[]) => {
          const data = args[1] as { fatal?: boolean } | undefined;
          if (data?.fatal) {
            setError("当前环境无法播放预告片");
            setPlaying(false);
            destroyPlayer();
          }
        });
        setPlaying(true);
        try {
          await el.play();
        } catch {
          setError("当前环境无法播放预告片");
          setPlaying(false);
          destroyPlayer();
        }
        return;
      }
    }

    setError("当前环境无法播放预告片");
    setPlaying(false);
  }, [destroyPlayer, item.hlsUrl, item.mp4Url]);

  return (
    <div className="gallery-stage-video">
      <video
        ref={videoRef}
        className="gallery-video"
        controls={playing}
        playsInline
        preload="metadata"
        poster={item.posterUrl}
        aria-label={item.title}
        onError={() => {
          if (playing) {
            setError("当前环境无法播放预告片");
            setPlaying(false);
            destroyPlayer();
          }
        }}
      />
      {!playing && (
        <button
          type="button"
          className="gallery-play-btn"
          onClick={() => void startPlayback()}
          aria-label={`播放 ${item.title}`}
        >
          <span className="gallery-play-icon" aria-hidden="true">
            ▶
          </span>
          <span>播放预告片</span>
        </button>
      )}
      {error && (
        <div className="gallery-video-error" role="status">
          <p>{error}</p>
          <a href={steamUrl} target="_blank" rel="noreferrer noopener">
            在 Steam 查看 ↗
          </a>
        </div>
      )}
    </div>
  );
}

export function GameMediaGallery({
  appId,
  name,
  coverUrl,
  media,
  steamUrl,
}: {
  appId: number;
  name: string;
  coverUrl: string | null;
  media?: GameMediaBlock | null;
  steamUrl: string;
}) {
  const items = useMemo(
    () => buildGalleryItems({ appId, name, coverUrl, media }),
    [appId, name, coverUrl, media],
  );

  const [index, setIndex] = useState(0);
  const [failedIds, setFailedIds] = useState<Set<string>>(() => new Set());
  const [lightboxOpen, setLightboxOpen] = useState(false);
  const thumbRefs = useRef<Array<HTMLButtonElement | null>>([]);

  const visibleItems = useMemo(
    () => items.filter((item) => !failedIds.has(item.id)),
    [items, failedIds],
  );

  useEffect(() => {
    setIndex(0);
    setFailedIds(new Set());
    setLightboxOpen(false);
  }, [appId, items]);

  useEffect(() => {
    if (index >= visibleItems.length) {
      setIndex(Math.max(0, visibleItems.length - 1));
    }
  }, [index, visibleItems.length]);

  const current = visibleItems[index] ?? null;

  const selectIndex = useCallback(
    (next: number) => {
      if (visibleItems.length === 0) return;
      const clamped = Math.max(0, Math.min(visibleItems.length - 1, next));
      setIndex(clamped);
      setLightboxOpen(false);
      thumbRefs.current[clamped]?.focus();
    },
    [visibleItems.length],
  );

  const markFailed = useCallback((id: string) => {
    setFailedIds((prev) => {
      if (prev.has(id)) return prev;
      const next = new Set(prev);
      next.add(id);
      return next;
    });
  }, []);

  const onRailKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (visibleItems.length === 0) return;
    if (event.key === "ArrowRight") {
      event.preventDefault();
      selectIndex(index + 1);
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      selectIndex(index - 1);
    } else if (event.key === "Home") {
      event.preventDefault();
      selectIndex(0);
    } else if (event.key === "End") {
      event.preventDefault();
      selectIndex(visibleItems.length - 1);
    }
  };

  if (!current) {
    // All candidates failed — letter placeholder like GameMedia.
    return (
      <div className="game-media-gallery" data-testid="game-media-gallery">
        <div className="gallery-stage">
          <div className="game-media gallery-fallback" aria-hidden="true">
            <span className="game-media-fallback">{name.slice(0, 1).toUpperCase()}</span>
          </div>
        </div>
      </div>
    );
  }

  const isImage = current.kind === "cover" || current.kind === "screenshot";
  const imageCurrent = isImage ? (current as GalleryImageItem) : null;
  const videoCurrent = current.kind === "video" ? current : null;

  return (
    <div className="game-media-gallery" data-testid="game-media-gallery">
      <div className="gallery-stage">
        {imageCurrent ? (
          <button
            type="button"
            className="gallery-stage-image"
            onClick={() => setLightboxOpen(true)}
            aria-label={`查看大图：${imageCurrent.alt}`}
          >
            <img
              src={imageCurrent.fullUrl}
              alt={imageCurrent.alt}
              loading="lazy"
              decoding="async"
              referrerPolicy="no-referrer"
              onError={() => markFailed(imageCurrent.id)}
            />
          </button>
        ) : videoCurrent ? (
          <VideoStage item={videoCurrent} steamUrl={steamUrl} active />
        ) : null}
      </div>

      {visibleItems.length > 1 && (
        <div
          className="gallery-rail"
          role="listbox"
          aria-label="媒体缩略图"
          onKeyDown={onRailKeyDown}
        >
          {visibleItems.map((item, i) => {
            const selected = i === index;
            const thumbSrc =
              item.kind === "video" ? item.posterUrl : item.thumbnailUrl;
            const label =
              item.kind === "video"
                ? `预告片：${item.title}`
                : item.kind === "cover"
                  ? "封面"
                  : `截图 ${i + 1}`;
            return (
              <button
                key={item.id}
                type="button"
                role="option"
                ref={(el) => {
                  thumbRefs.current[i] = el;
                }}
                className={`gallery-thumb${selected ? " is-current" : ""}${
                  item.kind === "video" ? " is-video" : ""
                }`}
                aria-label={label}
                aria-selected={selected}
                aria-current={selected ? "true" : undefined}
                onClick={() => selectIndex(i)}
              >
                <img
                  src={thumbSrc}
                  alt=""
                  loading="lazy"
                  decoding="async"
                  referrerPolicy="no-referrer"
                  onError={() => markFailed(item.id)}
                />
                {item.kind === "video" && (
                  <span className="gallery-thumb-play" aria-hidden="true">
                    ▶
                  </span>
                )}
              </button>
            );
          })}
        </div>
      )}

      {lightboxOpen && imageCurrent && (
        <Modal
          title={imageCurrent.alt}
          titleId="gallery-lightbox-title"
          onClose={() => setLightboxOpen(false)}
          className="gallery-lightbox"
        >
          <div className="gallery-lightbox-body">
            <img
              src={imageCurrent.fullUrl}
              alt={imageCurrent.alt}
              decoding="async"
              referrerPolicy="no-referrer"
            />
          </div>
        </Modal>
      )}
    </div>
  );
}
