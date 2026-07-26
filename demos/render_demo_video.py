"""Render a 45s Prove demo video from captured terminal logs."""
from __future__ import annotations

import re
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(r"C:\Users\Ayan Babwany\Desktop\AYAN FINAL CODING REPO 2025\2026 jul-aug job stuff\prove")
OUT = ROOT / "demos" / "output"
W, H = 1280, 720
FPS = 30
FFMPEG = r"C:\Users\Ayan Babwany\Downloads\ffmpeg-master-latest-win64-gpl-shared\ffmpeg-master-latest-win64-gpl-shared\bin\ffmpeg.exe"

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]|\x1b\].*?\x07|\x1b.")


def strip_ansi(s: str) -> str:
    return ANSI.sub("", s)


def load_lines(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = []
    for raw in text.splitlines():
        line = strip_ansi(raw).replace("\r", "").rstrip()
        # drop empty spam but keep structure
        if line.strip() == "":
            if lines and lines[-1] != "":
                lines.append("")
            continue
        lines.append(line[:110])
    return lines


def font(size: int, bold: bool = False):
    candidates = [
        r"C:\Windows\Fonts\consola.ttf",
        r"C:\Windows\Fonts\CascadiaMono.ttf",
        r"C:\Windows\Fonts\lucon.ttf",
        r"C:\Windows\Fonts\cour.ttf",
    ]
    if bold:
        candidates = [
            r"C:\Windows\Fonts\consolab.ttf",
            r"C:\Windows\Fonts\CascadiaMono.ttf",
        ] + candidates
    for c in candidates:
        p = Path(c)
        if p.exists():
            return ImageFont.truetype(str(p), size=size)
    return ImageFont.load_default()


F_TITLE = font(42, True)
F_SUB = font(22, True)
F_BODY = font(18)
F_SMALL = font(16)
F_METRIC = font(36, True)


def draw_terminal(draw: ImageDraw.ImageDraw, x, y, w, h, title: str, lines: list[str], scroll: int, accent):
    # window
    draw.rounded_rectangle([x, y, x + w, y + h], radius=14, fill=(18, 18, 24), outline=(50, 50, 60), width=2)
    draw.rounded_rectangle([x, y, x + w, y + 36], radius=14, fill=(28, 28, 36))
    draw.rectangle([x, y + 20, x + w, y + 36], fill=(28, 28, 36))
    for i, c in enumerate([(255, 95, 86), (255, 189, 46), (39, 201, 63)]):
        draw.ellipse([x + 14 + i * 18, y + 12, x + 26 + i * 18, y + 24], fill=c)
    draw.text((x + 80, y + 10), title, fill=accent, font=F_SMALL)

    visible = 22
    start = max(0, min(scroll, max(0, len(lines) - visible)))
    chunk = lines[start : start + visible]
    ty = y + 50
    for line in chunk:
        color = (200, 200, 210)
        low = line.lower()
        if "proof-or-stop" in low or "refusing" in low or "rejected" in low or "✗" in line or "failed" in low:
            color = (255, 120, 120)
        elif "✓" in line or "admitted" in low or "done" in low and "false" not in low:
            color = (120, 220, 150)
        elif "naive" in low or "self-reported" in low or "!" in line[:3]:
            color = (255, 200, 100)
        elif line.startswith("══") or line.startswith("==="):
            color = (140, 180, 255)
        draw.text((x + 16, ty), line, fill=color, font=F_BODY)
        ty += 22


def make_frame(t: float, naive: list[str], prove: list[str], eval_lines: list[str]) -> Image.Image:
    img = Image.new("RGB", (W, H), (10, 10, 14))
    d = ImageDraw.Draw(img)

    # header bar
    d.rectangle([0, 0, W, 78], fill=(14, 14, 20))
    d.text((40, 18), "Prove", fill=(240, 240, 245), font=F_TITLE)
    d.text((160, 30), "Don't trust the agent. Trust the evidence.", fill=(160, 170, 190), font=F_SUB)

    # timeline phases
    if t < 4:
        d.text((40, 100), "Coding agents can claim success.", fill=(220, 220, 230), font=F_SUB)
        d.text((40, 140), "Prove only advances on machine-checkable evidence.", fill=(160, 170, 190), font=F_SUB)
        d.text((40, 220), "github.com/AyanB123/prove", fill=(120, 160, 255), font=F_BODY)
    elif t < 22:
        # side by side scroll
        # naive finishes claims earlier; prove succeeds later
        n_scroll = int((t - 4) * 2.2)
        p_scroll = int(max(0, (t - 8)) * 2.0)
        draw_terminal(d, 30, 100, 600, 540, "naive backend  (self-report)", naive, n_scroll, (255, 180, 100))
        draw_terminal(d, 650, 100, 600, 540, "prove + local-loop  (evidence)", prove, p_scroll, (120, 220, 150))
        if t > 12:
            d.text((40, 660), "Left: claims tests passed  |  Right: hidden contract must pass", fill=(140, 140, 150), font=F_SMALL)
    elif t < 34:
        # eval
        e_scroll = int((t - 22) * 3.5)
        draw_terminal(d, 140, 110, 1000, 500, "prove eval traps", eval_lines, e_scroll, (140, 180, 255))
    else:
        # end card
        d.text((W // 2 - 280, 200), "false-done rate", fill=(160, 170, 190), font=F_SUB)
        d.text((W // 2 - 320, 260), "naive 10/10  →  prove 0/10", fill=(120, 220, 150), font=F_METRIC)
        d.text((W // 2 - 300, 340), "Agents can claim. Only evidence can advance.", fill=(220, 220, 230), font=F_SUB)
        d.text((W // 2 - 220, 420), "github.com/AyanB123/prove", fill=(120, 160, 255), font=F_SUB)
        d.text((W // 2 - 180, 480), "Apache-2.0  ·  prove eval traps", fill=(120, 120, 130), font=F_SMALL)

    # progress bar
    prog = min(1.0, t / 45.0)
    d.rectangle([0, H - 6, int(W * prog), H], fill=(80, 140, 255))
    return img


def main():
    naive = load_lines(OUT / "naive.txt")
    prove = load_lines(OUT / "local-loop.txt")
    eval_lines = load_lines(OUT / "eval.txt")
    # keep only the summary board for eval if long
    if "══ prove eval traps ══" in "\n".join(eval_lines):
        idx = next(i for i, l in enumerate(eval_lines) if "prove eval traps" in l)
        eval_lines = eval_lines[idx:]

    frames_dir = OUT / "frames"
    if frames_dir.exists():
        for p in frames_dir.glob("*.png"):
            p.unlink()
    else:
        frames_dir.mkdir(parents=True)

    duration = 45.0
    n = int(duration * FPS)
    print(f"rendering {n} frames...")
    for i in range(n):
        t = i / FPS
        frame = make_frame(t, naive, prove, eval_lines)
        frame.save(frames_dir / f"f{i:05d}.png")
        if i % 90 == 0:
            print(f"  {i}/{n}")

    mp4 = OUT / "prove-demo-45s.mp4"
    cmd = [
        FFMPEG,
        "-y",
        "-framerate",
        str(FPS),
        "-i",
        str(frames_dir / "f%05d.png"),
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-crf",
        "18",
        "-movflags",
        "+faststart",
        str(mp4),
    ]
    print("ffmpeg...", " ".join(cmd[-6:]))
    subprocess.check_call(cmd)
    print("wrote", mp4, "size", mp4.stat().st_size)

    # also a short gif preview (first 8s at lower fps)
    gif = OUT / "prove-demo-preview.gif"
    # sample every 6th frame for first 8 seconds
    gif_frames = []
    for i in range(0, int(8 * FPS), 6):
        gif_frames.append(Image.open(frames_dir / f"f{i:05d}.png").resize((640, 360)))
    if gif_frames:
        gif_frames[0].save(
            gif,
            save_all=True,
            append_images=gif_frames[1:],
            duration=200,
            loop=0,
            optimize=True,
        )
        print("wrote", gif, "size", gif.stat().st_size)


if __name__ == "__main__":
    main()
